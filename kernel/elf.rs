//! Bounded ELF64 loader for the position-independent user programs.
//!
//! This is a Rust-native replacement for the supplied C loader
//! (csci1670/cs1670-f26-elfloader). Unlike the C version, which only copies
//! `PT_LOAD` file bytes, this loader also zero-fills the `bss` tail of each
//! segment and applies the `R_AARCH64_RELATIVE` relocations a PIE binary
//! needs, because Rust-compiled user programs are linked at offset 0 and must
//! be relocated to the actual base address they are loaded at.
//!
//! `load_elf` does not execute the loaded code and does not choose a physical
//! address: the caller picks where in RAM the program should live, hands us an
//! exclusive `dest` slice covering the program's `[0, memsz)` virtual range,
//! and later jumps to the returned entry point.
//!
//! Supported: little-endian ELF64, AArch64, `ET_DYN`, `PT_LOAD` segments and a
//! `PT_DYNAMIC` table containing only `DT_RELA`/`RELASZ`/`RELAENT`(/`RELACOUNT`)
//! descriptors plus ignored metadata tags. Anything else (interpreter, TLS,
//! shared-library dependencies, non-RELATIVE relocations) is rejected: this is
//! not a general dynamic linker.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    InvalidHeader,
    Unsupported,
    Truncated,
    OutOfBounds,
    InvalidSegment,
    InvalidRelocation,
    MissingLoadSegment,
    InvalidEntry,
}

// ELF64 constants.
const EHDR_SIZE: usize = 64;
const PHDR_SIZE: usize = 56;
const EI_CLASS_64: u8 = 2;
const EI_DATA_LE: u8 = 1;
const EV_CURRENT: u32 = 1;
const ET_DYN: u16 = 3;
const EM_AARCH64: u16 = 183;

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PT_INTERP: u32 = 3;
const PT_TLS: u32 = 7;

const PF_X: u32 = 1;
const PF_W: u32 = 2;

const DT_NULL: i64 = 0;
const DT_NEEDED: i64 = 1;
const DT_REL: i64 = 17;
const DT_RELSZ: i64 = 18;
const DT_RELENT: i64 = 19;
const DT_TEXTREL: i64 = 22;
const DT_JMPREL: i64 = 23;
const DT_RELA: i64 = 7;
const DT_RELASZ: i64 = 8;
const DT_RELAENT: i64 = 9;
const DT_RELACOUNT: i64 = 0x6fff_fff9;
const DT_RELRSZ: i64 = 35;
const DT_RELR: i64 = 36;
const DT_RELRENT: i64 = 37;

const RELA_SIZE: usize = 24;
const R_AARCH64_RELATIVE_INFO: u64 = 1027; // sym 0, type R_AARCH64_RELATIVE

#[derive(Clone, Copy)]
struct Phdr {
    p_type: u32,
    flags: u32,
    offset: u64,
    vaddr: u64,
    filesz: u64,
    memsz: u64,
    align: u64,
}

fn u16_at(b: &[u8], off: usize) -> Result<u16, ElfError> {
    let bytes: [u8; 2] = b
        .get(off..off + 2)
        .ok_or(ElfError::Truncated)?
        .try_into()
        .map_err(|_| ElfError::Truncated)?;
    Ok(u16::from_le_bytes(bytes))
}

fn u32_at(b: &[u8], off: usize) -> Result<u32, ElfError> {
    let bytes: [u8; 4] = b
        .get(off..off + 4)
        .ok_or(ElfError::Truncated)?
        .try_into()
        .map_err(|_| ElfError::Truncated)?;
    Ok(u32::from_le_bytes(bytes))
}

fn u64_at(b: &[u8], off: usize) -> Result<u64, ElfError> {
    let bytes: [u8; 8] = b
        .get(off..off + 8)
        .ok_or(ElfError::Truncated)?
        .try_into()
        .map_err(|_| ElfError::Truncated)?;
    Ok(u64::from_le_bytes(bytes))
}

fn i64_at(b: &[u8], off: usize) -> Result<i64, ElfError> {
    Ok(u64_at(b, off)? as i64)
}

/// Load `image` (an ELF64 file) into `dest`, which must cover the image's
/// `[0, highest_vaddr + memsz)` virtual address range. Returns the relocated
/// entry point (as `dest` base + `e_entry`).
///
/// Safety-wise this only writes inside `dest`; actually jumping to the
/// returned pointer is up to the caller.
pub fn load_elf(image: &[u8], dest: &mut [u8]) -> Result<*const (), ElfError> {
    // ---- Header validation -------------------------------------------------
    if image.len() < EHDR_SIZE {
        return Err(ElfError::Truncated);
    }
    if image[0..4] != [0x7f, b'E', b'L', b'F']
        || image[4] != EI_CLASS_64
        || image[5] != EI_DATA_LE
        || image[6] != 1
    {
        return Err(ElfError::InvalidHeader);
    }
    let e_type = u16_at(image, 16)?;
    let e_machine = u16_at(image, 18)?;
    let e_version = u32_at(image, 20)?;
    if e_machine != EM_AARCH64 || e_version != EV_CURRENT {
        return Err(ElfError::InvalidHeader);
    }
    if e_type != ET_DYN {
        // We only load zero-linked position-independent executables.
        return Err(ElfError::Unsupported);
    }
    let e_entry = u64_at(image, 24)?;
    let e_phoff = u64_at(image, 32)? as usize;
    let e_ehsize = u16_at(image, 52)? as usize;
    let e_phentsize = u16_at(image, 54)? as usize;
    let e_phnum = u16_at(image, 56)? as usize;
    if e_ehsize != EHDR_SIZE || e_phentsize != PHDR_SIZE {
        return Err(ElfError::InvalidHeader);
    }
    let ph_table_end = e_phoff
        .checked_add(e_phnum.checked_mul(PHDR_SIZE).ok_or(ElfError::Truncated)?)
        .ok_or(ElfError::Truncated)?;
    if ph_table_end > image.len() {
        return Err(ElfError::Truncated);
    }

    // ---- Program header scan + validation ----------------------------------
    let mut loads: [Option<Phdr>; 32] = [None; 32];
    let mut nloads = 0usize;
    let mut dynamic: Option<Phdr> = None;

    for i in 0..e_phnum {
        let off = e_phoff + i * PHDR_SIZE;
        let ph = Phdr {
            p_type: u32_at(image, off)?,
            flags: u32_at(image, off + 4)?,
            offset: u64_at(image, off + 8)?,
            vaddr: u64_at(image, off + 16)?,
            filesz: u64_at(image, off + 32)?,
            memsz: u64_at(image, off + 40)?,
            align: u64_at(image, off + 48)?,
        };
        match ph.p_type {
            PT_LOAD => {
                if ph.filesz > ph.memsz {
                    return Err(ElfError::InvalidSegment);
                }
                // File range must be inside the image; virtual range inside dest.
                let file_end = ph
                    .offset
                    .checked_add(ph.filesz)
                    .ok_or(ElfError::OutOfBounds)?;
                if file_end > image.len() as u64 {
                    return Err(ElfError::OutOfBounds);
                }
                let v_end = ph
                    .vaddr
                    .checked_add(ph.memsz)
                    .ok_or(ElfError::OutOfBounds)?;
                if v_end > dest.len() as u64 {
                    return Err(ElfError::OutOfBounds);
                }
                if ph.align > 1 {
                    if !ph.align.is_power_of_two() {
                        return Err(ElfError::InvalidSegment);
                    }
                    // File offset and virtual address must be congruent, and
                    // the load base must satisfy the segment alignment.
                    if ph.vaddr % ph.align != ph.offset % ph.align {
                        return Err(ElfError::InvalidSegment);
                    }
                    if (dest.as_ptr() as u64) % ph.align != 0 {
                        return Err(ElfError::InvalidSegment);
                    }
                }
                if nloads == loads.len() {
                    return Err(ElfError::Unsupported);
                }
                loads[nloads] = Some(ph);
                nloads += 1;
            }
            PT_DYNAMIC => {
                if dynamic.is_some() || ph.filesz > ph.memsz {
                    return Err(ElfError::InvalidSegment);
                }
                let file_end = ph
                    .offset
                    .checked_add(ph.filesz)
                    .ok_or(ElfError::OutOfBounds)?;
                if file_end > image.len() as u64 {
                    return Err(ElfError::OutOfBounds);
                }
                let vend = ph
                    .vaddr
                    .checked_add(ph.memsz)
                    .ok_or(ElfError::OutOfBounds)?;
                if vend > dest.len() as u64 {
                    return Err(ElfError::OutOfBounds);
                }
                dynamic = Some(ph);
            }
            PT_INTERP | PT_TLS => return Err(ElfError::Unsupported),
            _ => {}
        }
    }
    if nloads == 0 {
        return Err(ElfError::MissingLoadSegment);
    }

    // Reject overlapping memory segments: ambiguous which bytes win.
    let mut segs: [Phdr; 32] = [Phdr {
        p_type: 0,
        flags: 0,
        offset: 0,
        vaddr: 0,
        filesz: 0,
        memsz: 0,
        align: 1,
    }; 32];
    for i in 0..nloads {
        segs[i] = loads[i].unwrap();
    }
    let segs = &mut segs[..nloads];
    segs.sort_unstable_by_key(|s| s.vaddr);
    for w in segs.windows(2) {
        if w[0].vaddr + w[0].memsz > w[1].vaddr {
            return Err(ElfError::InvalidSegment);
        }
    }

    // PT_DYNAMIC must actually map file-backed bytes inside one PT_LOAD.
    if let Some(d) = dynamic {
        if !segs.iter().any(|s| {
            d.vaddr >= s.vaddr
                && d.vaddr
                    .checked_add(d.memsz)
                    .is_some_and(|e| e <= s.vaddr + s.memsz)
                && d.vaddr
                    .checked_add(d.filesz)
                    .is_some_and(|e| e <= s.vaddr + s.filesz)
                && s.offset.checked_add(d.vaddr - s.vaddr) == Some(d.offset)
        }) {
            return Err(ElfError::InvalidSegment);
        }
    }

    // Entry must be 4-byte aligned (in the final loaded image, too) and the
    // whole first instruction must sit inside an executable file-backed
    // PT_LOAD.
    let base = dest.as_ptr() as u64;
    let entry = base.checked_add(e_entry).ok_or(ElfError::InvalidEntry)?;
    if e_entry % 4 != 0
        || entry % 4 != 0
        || !segs.iter().any(|s| {
            s.flags & PF_X != 0
                && e_entry >= s.vaddr
                && e_entry
                    .checked_add(4)
                    .is_some_and(|e| e <= s.vaddr + s.filesz)
        })
    {
        return Err(ElfError::InvalidEntry);
    }

    // ---- Dynamic table scan (validation before we start writing) -----------
    // Parse the dynamic table for the RELA descriptors. The table lives at a
    // virtual address inside a PT_LOAD; find it there so we can also compute
    // where it lands in `dest`.
    let mut rela_vaddr: Option<u64> = None;
    let mut rela_sz: Option<u64> = None;
    let mut rela_ent: Option<u64> = None;
    if let Some(dyn_ph) = dynamic {
        // Dynamic entries must be readable from the file image.
        let dstart = dyn_ph.offset as usize;
        let dend = (dyn_ph.offset + dyn_ph.filesz) as usize;
        let mut off = dstart;
        loop {
            if off + 16 > dend {
                // Ran off the end without DT_NULL.
                return Err(ElfError::InvalidRelocation);
            }
            let tag = i64_at(image, off)?;
            let val = u64_at(image, off + 8)?;
            off += 16;
            match tag {
                DT_NULL => break,
                DT_RELA => rela_vaddr = Some(val),
                DT_RELASZ => rela_sz = Some(val),
                DT_RELAENT => rela_ent = Some(val),
                DT_RELACOUNT => {}
                DT_NEEDED | DT_REL | DT_RELSZ | DT_RELENT | DT_TEXTREL | DT_JMPREL | DT_RELRSZ
                | DT_RELR | DT_RELRENT => return Err(ElfError::Unsupported),
                _ => {}
            }
        }
    }
    // All-or-nothing: a partially described RELA table is malformed.
    let rela = match (rela_vaddr, rela_sz, rela_ent) {
        (None, None, None) => None,
        (Some(v), Some(sz), Some(ent)) => {
            if ent != RELA_SIZE as u64 || sz % RELA_SIZE as u64 != 0 {
                return Err(ElfError::InvalidRelocation);
            }
            Some((v, sz))
        }
        _ => return Err(ElfError::InvalidRelocation),
    };

    // Helper: does [vaddr, vaddr+len) fall inside a PT_LOAD with `flag`?
    let in_segment = |vaddr: u64, len: u64, flag: u32| -> bool {
        segs.iter().any(|s| {
            s.flags & flag != 0
                && vaddr >= s.vaddr
                && vaddr
                    .checked_add(len)
                    .is_some_and(|e| e <= s.vaddr + s.memsz)
        })
    };
    let in_any_load = |vaddr: u64, len: u64| -> bool {
        segs.iter().any(|s| {
            vaddr >= s.vaddr
                && vaddr
                    .checked_add(len)
                    .is_some_and(|e| e <= s.vaddr + s.memsz)
        })
    };

    // Metadata ranges (in dest offsets) that relocation writes must not hit,
    // since we read them while applying fixups.
    let mut protected: [(u64, u64); 2] = [(0, 0); 2];
    let mut nprotected = 0usize;
    if let Some(dyn_ph) = dynamic {
        protected[nprotected] = (dyn_ph.vaddr, dyn_ph.memsz);
        nprotected += 1;
    }
    if let Some((rv, rsz)) = rela {
        // The table must lie entirely in loaded (mapped) memory.
        if !in_any_load(rv, rsz) {
            return Err(ElfError::InvalidRelocation);
        }
        protected[nprotected] = (rv, rsz);
        nprotected += 1;

        // Validate every entry up front (still before any writes).
        for eoff in (rv..rv + rsz).step_by(RELA_SIZE) {
            // eoff is a virtual address; read via the file image if it maps a
            // file range, else it must be in a bss tail — but a bss table is
            // meaningless, so require file-backed bytes.
            let file_pos = segs.iter().find_map(|s| {
                if eoff >= s.vaddr && eoff + RELA_SIZE as u64 <= s.vaddr + s.filesz {
                    Some(s.offset + (eoff - s.vaddr))
                } else {
                    None
                }
            });
            let fp = file_pos.ok_or(ElfError::InvalidRelocation)? as usize;
            let r_offset = u64_at(image, fp)?;
            let r_info = u64_at(image, fp + 8)?;
            let r_addend = i64_at(image, fp + 16)?;
            if r_info != R_AARCH64_RELATIVE_INFO {
                return Err(ElfError::InvalidRelocation);
            }
            // Target must be an 8-byte range in a writable loaded segment.
            if r_offset % 8 != 0 || !in_segment(r_offset, 8, PF_W) {
                return Err(ElfError::InvalidRelocation);
            }
            // It must not overwrite the dynamic/RELA metadata we are reading.
            // (pv+plen is bounded: both were validated inside loaded segments.)
            for &(pv, plen) in &protected[..nprotected] {
                if plen != 0 && r_offset < pv + plen && pv < r_offset + 8 {
                    return Err(ElfError::InvalidRelocation);
                }
            }
            // The addend is an offset from the load base: non-negative and
            // pointing into a loaded segment (a one-past-the-end address is
            // allowed via len 0), with the final relocated value computed
            // without overflow.
            if r_addend < 0
                || !in_any_load(r_addend as u64, 0)
                || base.checked_add(r_addend as u64).is_none()
            {
                return Err(ElfError::InvalidRelocation);
            }
        }
    }

    // ---- Copy segments + zero bss ------------------------------------------
    for s in segs.iter() {
        let dst = &mut dest[s.vaddr as usize..(s.vaddr + s.memsz) as usize];
        dst[..s.filesz as usize]
            .copy_from_slice(&image[s.offset as usize..(s.offset + s.filesz) as usize]);
        dst[s.filesz as usize..].fill(0);
    }

    // ---- Apply relocations --------------------------------------------------
    if let Some((rv, rsz)) = rela {
        for eoff in (rv..rv + rsz).step_by(RELA_SIZE) {
            let fp = segs
                .iter()
                .find_map(|s| {
                    if eoff >= s.vaddr && eoff + RELA_SIZE as u64 <= s.vaddr + s.filesz {
                        Some(s.offset + (eoff - s.vaddr))
                    } else {
                        None
                    }
                })
                .ok_or(ElfError::InvalidRelocation)? as usize;
            let r_offset = u64_at(image, fp)? as usize;
            let r_addend = u64_at(image, fp + 16)? as i64 as u64;
            let value = base
                .checked_add(r_addend)
                .ok_or(ElfError::InvalidRelocation)?;
            dest[r_offset..r_offset + 8].copy_from_slice(&value.to_le_bytes());
        }
    }

    Ok(entry as *const ())
}
