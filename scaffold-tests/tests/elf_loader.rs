// Host-side tests for the bounded ELF64 loader in kernel/elf.rs.

#[path = "../../kernel/elf.rs"]
mod elf;

use elf::{ElfError, load_elf};

// ---- Synthetic ELF builder ------------------------------------------------

#[derive(Clone)]
struct Seg {
    p_type: u32,
    flags: u32,
    offset: u64,
    vaddr: u64,
    filesz: u64,
    memsz: u64,
    align: u64,
}

struct ElfImg {
    bytes: Vec<u8>,
}

fn base_header() -> Vec<u8> {
    let mut h = vec![0u8; 64];
    h[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
    h[4] = 2; // ELFCLASS64
    h[5] = 1; // little-endian
    h[6] = 1; // EI_VERSION
    h[16..18].copy_from_slice(&3u16.to_le_bytes()); // ET_DYN
    h[18..20].copy_from_slice(&183u16.to_le_bytes()); // EM_AARCH64
    h[20..24].copy_from_slice(&1u32.to_le_bytes()); // EV_CURRENT
    h[52..54].copy_from_slice(&64u16.to_le_bytes()); // e_ehsize
    h[54..56].copy_from_slice(&56u16.to_le_bytes()); // e_phentsize
    h
}

/// Build an ELF: `segs` program headers (file bytes drawn from `body`, which
/// is appended after the program header table), entry address `entry`.
fn build(segs: &[Seg], body: &[(u64, &[u8])], entry: u64) -> ElfImg {
    let mut img = base_header();
    let phoff = 64u64;
    img[32..40].copy_from_slice(&phoff.to_le_bytes()); // e_phoff
    img[24..32].copy_from_slice(&entry.to_le_bytes()); // e_entry
    img[56..58].copy_from_slice(&(segs.len() as u16).to_le_bytes()); // e_phnum
    let mut size = phoff + 56 * segs.len() as u64;
    for s in segs {
        size = size.max(s.offset + s.filesz);
    }
    for (off, data) in body {
        size = size.max(off + data.len() as u64);
    }
    img.resize(size as usize, 0);
    for (i, s) in segs.iter().enumerate() {
        let o = (phoff + 56 * i as u64) as usize;
        img[o..o + 4].copy_from_slice(&s.p_type.to_le_bytes());
        img[o + 4..o + 8].copy_from_slice(&s.flags.to_le_bytes());
        img[o + 8..o + 16].copy_from_slice(&s.offset.to_le_bytes());
        img[o + 16..o + 24].copy_from_slice(&s.vaddr.to_le_bytes());
        img[o + 24..o + 32].copy_from_slice(&s.vaddr.to_le_bytes()); // paddr
        img[o + 32..o + 40].copy_from_slice(&s.filesz.to_le_bytes());
        img[o + 40..o + 48].copy_from_slice(&s.memsz.to_le_bytes());
        img[o + 48..o + 56].copy_from_slice(&s.align.to_le_bytes());
    }
    for (off, data) in body {
        img[*off as usize..*off as usize + data.len()].copy_from_slice(data);
    }
    ElfImg { bytes: img }
}

fn seg(p_type: u32, flags: u32, offset: u64, vaddr: u64, filesz: u64, memsz: u64) -> Seg {
    Seg {
        p_type,
        flags,
        offset,
        vaddr,
        filesz,
        memsz,
        align: 1,
    }
}

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;

const SENTINEL: u8 = 0xa5;

/// 16-byte-aligned backing buffer, prefilled with the sentinel so the tests
/// catch both missing writes and writes outside the loaded segments.
fn dest_backing(n: usize) -> Vec<u128> {
    vec![0xa5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5u128; n.div_ceil(16)]
}

fn dest_of(backing: &mut [u128], n: usize) -> &mut [u8] {
    assert!(n <= core::mem::size_of_val(backing));
    unsafe { core::slice::from_raw_parts_mut(backing.as_mut_ptr().cast::<u8>(), n) }
}

/// Minimal good image: R+X segment (16 bytes code at vaddr 0), RW segment
/// (8 bytes file + 8 bytes bss at vaddr 0x1000). Entry = 0.
/// Total span: 0x1010 bytes.
fn good_image() -> (ElfImg, Vec<u8>) {
    let code: &[u8] = &[
        0x1f, 0x20, 0x03, 0xd5, 0xc0, 0x03, 0x5f, 0xd6, 1, 2, 3, 4, 5, 6, 7, 8,
    ];
    let data: &[u8] = &[0xaa, 0xbb, 0xcc, 0xdd, 0x11, 0x22, 0x33, 0x44];
    let img = build(
        &[
            seg(PT_LOAD, PF_R | PF_X, 0x100, 0x0, 16, 16),
            seg(PT_LOAD, PF_R | PF_W, 0x110, 0x1000, 8, 16),
        ],
        &[(0x100, code), (0x110, data)],
        0,
    );
    // Expected dest: sentinels everywhere except copied bytes and zeroed bss.
    let mut expected = vec![SENTINEL; 0x1010];
    expected[..16].copy_from_slice(code);
    expected[0x1000..0x1008].copy_from_slice(data);
    expected[0x1008..0x1010].fill(0);
    (img, expected)
}

#[test]
fn loads_good_image_copies_and_zeros_bss() {
    let (img, expected) = good_image();
    let mut backing = dest_backing(0x2000);
    let dest = dest_of(&mut backing, 0x1010);
    let entry = load_elf(&img.bytes, dest).unwrap();
    assert_eq!(entry, dest.as_ptr() as *const ());
    assert_eq!(dest, &expected[..]);
}

#[test]
fn exact_capacity_succeeds_and_one_byte_short_fails() {
    let (img, _) = good_image();
    let mut backing = dest_backing(0x2000);
    let dest = dest_of(&mut backing, 0x1010);
    load_elf(&img.bytes, dest).unwrap();

    let mut backing = dest_backing(0x2000);
    let dest = dest_of(&mut backing, 0x100f); // one byte short of span
    assert!(load_elf(&img.bytes, dest).is_err());
    assert!(dest.iter().all(|&b| b == SENTINEL));
}

#[test]
fn unaligned_image_slice_ok() {
    let (img, expected) = good_image();
    let mut padded = vec![0u8; img.bytes.len() + 1];
    padded[1..].copy_from_slice(&img.bytes);
    let mut backing = dest_backing(0x2000);
    let dest = dest_of(&mut backing, 0x1010);
    load_elf(&padded[1..], dest).unwrap();
    assert_eq!(dest, &expected[..]);
}

#[test]
fn rejects_truncated() {
    let (img, _) = good_image();
    let mut backing = dest_backing(0x2000);
    let dest = dest_of(&mut backing, 0x1010);
    // Truncated header
    assert!(load_elf(&img.bytes[..32], dest).is_err());
    // Truncated program header table
    assert!(load_elf(&img.bytes[..64 + 60], dest).is_err());
    // Truncated file bytes
    assert!(load_elf(&img.bytes[..0x110], dest).is_err());
    assert!(dest.iter().all(|&b| b == SENTINEL));
}

#[test]
fn rejects_bad_header_fields() {
    let (img, _) = good_image();
    let mut backing = dest_backing(0x2000);
    let dest = dest_of(&mut backing, 0x1010);
    for (off, val) in [
        (4usize, 1u8), // ELFCLASS32
        (5, 2),        // big-endian
        (6, 0),        // bad EI_VERSION
    ] {
        let mut bad = img.bytes.clone();
        bad[off] = val;
        assert_eq!(load_elf(&bad, dest), Err(ElfError::InvalidHeader));
    }
    let mut bad = img.bytes.clone(); // bad magic
    bad[0] = 0;
    assert_eq!(load_elf(&bad, dest), Err(ElfError::InvalidHeader));
    let mut bad = img.bytes.clone(); // x86-64
    bad[18..20].copy_from_slice(&62u16.to_le_bytes());
    assert_eq!(load_elf(&bad, dest), Err(ElfError::InvalidHeader));
    let mut bad = img.bytes.clone(); // bad e_version
    bad[20..24].copy_from_slice(&2u32.to_le_bytes());
    assert_eq!(load_elf(&bad, dest), Err(ElfError::InvalidHeader));
    let mut bad = img.bytes.clone(); // ET_EXEC
    bad[16..18].copy_from_slice(&2u16.to_le_bytes());
    assert_eq!(load_elf(&bad, dest), Err(ElfError::Unsupported));
    // phnum = 0 -> no load segments
    let mut bad = img.bytes.clone();
    bad[56..58].copy_from_slice(&0u16.to_le_bytes());
    assert_eq!(load_elf(&bad, dest), Err(ElfError::MissingLoadSegment));
    assert!(dest.iter().all(|&b| b == SENTINEL));
}

#[test]
fn rejects_bad_segments() {
    let mut backing = dest_backing(0x2000);
    let dest = dest_of(&mut backing, 0x1010);
    // filesz > memsz
    let img = build(&[seg(PT_LOAD, PF_R | PF_X, 0x100, 0, 16, 8)], &[], 0);
    assert_eq!(load_elf(&img.bytes, dest), Err(ElfError::InvalidSegment));
    // past end of dest
    let img = build(&[seg(PT_LOAD, PF_R | PF_X, 0x100, 0, 16, 0x3000)], &[], 0);
    assert_eq!(load_elf(&img.bytes, dest), Err(ElfError::OutOfBounds));
    // overlapping memory segments
    let img = build(
        &[
            seg(PT_LOAD, PF_R | PF_X, 0x100, 0, 16, 16),
            seg(PT_LOAD, PF_R | PF_W, 0x110, 8, 8, 8),
        ],
        &[],
        0,
    );
    assert_eq!(load_elf(&img.bytes, dest), Err(ElfError::InvalidSegment));
    // no PT_LOAD
    let img = build(&[seg(PT_DYNAMIC, PF_R, 0x100, 0, 16, 16)], &[], 0);
    assert_eq!(
        load_elf(&img.bytes, dest),
        Err(ElfError::MissingLoadSegment)
    );
    // entry in non-executable segment
    let img = build(
        &[
            seg(PT_LOAD, PF_R | PF_X, 0x100, 0, 16, 16),
            seg(PT_LOAD, PF_R | PF_W, 0x110, 0x1000, 8, 8),
        ],
        &[],
        0x1000,
    );
    assert_eq!(load_elf(&img.bytes, dest), Err(ElfError::InvalidEntry));
    // misaligned entry
    let img = build(&[seg(PT_LOAD, PF_R | PF_X, 0x100, 0, 16, 16)], &[], 2);
    assert_eq!(load_elf(&img.bytes, dest), Err(ElfError::InvalidEntry));
    // entry in the bss tail past the file-backed bytes (instruction truncated)
    let img = build(&[seg(PT_LOAD, PF_R | PF_X, 0x100, 0, 16, 0x100)], &[], 16);
    assert_eq!(load_elf(&img.bytes, dest), Err(ElfError::InvalidEntry));
    // p_align = 3 (not a power of two)
    let mut s = seg(PT_LOAD, PF_R | PF_X, 0x100, 0, 16, 16);
    s.align = 3;
    let img = build(&[s], &[], 0);
    assert_eq!(load_elf(&img.bytes, dest), Err(ElfError::InvalidSegment));
    // incongruent offset vs vaddr under p_align
    let mut s = seg(PT_LOAD, PF_R | PF_X, 0x104, 0, 16, 16);
    s.align = 16;
    let img = build(&[s], &[], 0);
    assert_eq!(load_elf(&img.bytes, dest), Err(ElfError::InvalidSegment));
    // ph.offset + filesz overflow (mutated, no huge allocation)
    let (img2, _) = good_image();
    let mut bad = img2.bytes.clone();
    bad[64 + 8..64 + 16].copy_from_slice(&u64::MAX.to_le_bytes()); // p_offset
    assert!(load_elf(&bad, dest).is_err());
    // ph.vaddr + memsz overflow
    let mut bad = img2.bytes.clone();
    bad[64 + 16..64 + 24].copy_from_slice(&u64::MAX.to_le_bytes()); // p_vaddr
    assert!(load_elf(&bad, dest).is_err());
    assert!(dest.iter().all(|&b| b == SENTINEL));
}

/// Image with a PT_DYNAMIC table holding one R_AARCH64_RELATIVE entry whose
/// target is the 8-byte slot at vaddr 0x1000.
fn rela_image(r_info: u64, addend: i64, rela_vaddr: u64, rela_in_file: bool) -> ElfImg {
    let mut dyn_table = Vec::new();
    dyn_table.extend_from_slice(&7i64.to_le_bytes()); // DT_RELA
    dyn_table.extend_from_slice(&rela_vaddr.to_le_bytes());
    dyn_table.extend_from_slice(&8i64.to_le_bytes()); // DT_RELASZ
    dyn_table.extend_from_slice(&24u64.to_le_bytes());
    dyn_table.extend_from_slice(&9i64.to_le_bytes()); // DT_RELAENT
    dyn_table.extend_from_slice(&24u64.to_le_bytes());
    dyn_table.extend_from_slice(&0i64.to_le_bytes()); // DT_NULL
    dyn_table.extend_from_slice(&0u64.to_le_bytes());

    let mut rela = Vec::new();
    rela.extend_from_slice(&0x1000u64.to_le_bytes()); // r_offset
    rela.extend_from_slice(&r_info.to_le_bytes());
    rela.extend_from_slice(&addend.to_le_bytes());

    let dyn_vaddr = 0x2000u64;
    let dyn_foff = 0x300u64;
    let rela_foff = 0x340u64;

    let mut body: Vec<(u64, &[u8])> = vec![
        (0x100, &[0x1f, 0x20, 0x03, 0xd5]),
        (0x200, &[0; 8]),
        (dyn_foff, &dyn_table),
    ];
    if rela_in_file {
        body.push((rela_foff, &rela));
    }
    build(
        &[
            seg(PT_LOAD, PF_R | PF_X, 0x100, 0, 4, 4),
            seg(PT_LOAD, PF_R | PF_W, 0x200, 0x1000, 8, 16),
            Seg {
                align: 8,
                ..seg(PT_LOAD, PF_R | PF_W, 0x300, dyn_vaddr, 0x80, 0x80)
            },
            seg(
                PT_DYNAMIC,
                PF_R | PF_W,
                dyn_foff,
                dyn_vaddr,
                dyn_table.len() as u64,
                dyn_table.len() as u64,
            ),
        ],
        &body,
        0,
    )
}

#[test]
fn applies_relative_relocations() {
    let img = rela_image(1027, 0x1000, 0x2040, true);
    let mut backing = dest_backing(0x4000);
    let dest = dest_of(&mut backing, 0x3000);
    let base = dest.as_ptr() as u64;
    load_elf(&img.bytes, dest).unwrap();
    let got = u64::from_le_bytes(dest[0x1000..0x1008].try_into().unwrap());
    assert_eq!(got, base + 0x1000);
}

#[test]
fn rejects_bad_relocations() {
    let mut backing = dest_backing(0x4000);
    let dest = dest_of(&mut backing, 0x3000);
    // Unsupported relocation type (symbol / non-RELATIVE)
    let img = rela_image((5 << 32) | 1027, 0x1000, 0x2040, true);
    assert_eq!(load_elf(&img.bytes, dest), Err(ElfError::InvalidRelocation));
    let img = rela_image(257, 0x1000, 0x2040, true); // R_AARCH64_ABS64
    assert_eq!(load_elf(&img.bytes, dest), Err(ElfError::InvalidRelocation));
    // Negative addend
    let img = rela_image(1027, -4, 0x2040, true);
    assert_eq!(load_elf(&img.bytes, dest), Err(ElfError::InvalidRelocation));
    // Addend into unmapped gap (0x800 is not inside any PT_LOAD)
    let img = rela_image(1027, 0x800, 0x2040, true);
    assert_eq!(load_elf(&img.bytes, dest), Err(ElfError::InvalidRelocation));
    // RELA table outside any loaded segment
    let img = rela_image(1027, 0x1000, 0x9000, true);
    assert_eq!(load_elf(&img.bytes, dest), Err(ElfError::InvalidRelocation));
    assert!(dest.iter().all(|&b| b == SENTINEL));
}

#[test]
fn rejects_malformed_relocation_ranges() {
    let mut backing = dest_backing(0x4000);
    let dest = dest_of(&mut backing, 0x3000);
    let img = rela_image(1027, 0x1000, 0x2040, true);
    // Mutate r_offset (file offset 0x340) to targets that must all fail:
    for r_offset in [
        0x9000u64, // outside all loaded segments
        0x0,       // text segment is not writable
        0x2000,    // dynamic metadata (protected range)
        0x2040,    // rela metadata (protected range)
        0x1001,    // unaligned target
    ] {
        let mut bad = img.bytes.clone();
        bad[0x340..0x348].copy_from_slice(&r_offset.to_le_bytes());
        assert_eq!(
            load_elf(&bad, dest),
            Err(ElfError::InvalidRelocation),
            "r_offset {r_offset:#x}"
        );
    }
    // RELA table landing in bss: shrink the third PT_LOAD's filesz to 0x40
    // (dynamic table at vaddr 0x2000 stays file-backed; the rela table at
    // vaddr 0x2040 moves into the bss tail).
    let mut bad = img.bytes.clone();
    bad[64 + 2 * 56 + 32..64 + 2 * 56 + 40].copy_from_slice(&0x40u64.to_le_bytes());
    assert_eq!(load_elf(&bad, dest), Err(ElfError::InvalidRelocation));
    // Misaligned actual destination with p_align = 1 (full capacity, but
    // the slice starts at an odd address so base + e_entry is misaligned)
    let (good, _) = good_image();
    let mut backing = dest_backing(0x2000);
    let dest = dest_of(&mut backing, 0x1011);
    assert_eq!(
        load_elf(&good.bytes, &mut dest[1..]),
        Err(ElfError::InvalidEntry)
    );
    assert!(dest.iter().all(|&b| b == SENTINEL));
}

#[test]
fn rejects_bad_dynamic() {
    let (img, _) = good_image();
    let mut backing = dest_backing(0x4000);
    let dest = dest_of(&mut backing, 0x3000);

    // Duplicate PT_DYNAMIC
    let img2 = rela_image(1027, 0x1000, 0x2040, true);
    let mut bad = img2.bytes.clone();
    let mut phnum = u16::from_le_bytes(bad[56..58].try_into().unwrap());
    phnum += 1;
    bad[56..58].copy_from_slice(&phnum.to_le_bytes());
    bad.resize(bad.len() + 56, 0);
    // copy the PT_DYNAMIC phdr (index 3) into the new slot (index 4)
    bad.copy_within(64 + 3 * 56..64 + 4 * 56, 64 + 4 * 56);
    assert_eq!(load_elf(&bad, dest), Err(ElfError::InvalidSegment));

    // PT_DYNAMIC vaddr outside any PT_LOAD
    let mut bad = img2.bytes.clone();
    bad[64 + 3 * 56 + 16..64 + 3 * 56 + 24].copy_from_slice(&0x9000u64.to_le_bytes());
    assert!(load_elf(&bad, dest).is_err());
    // PT_DYNAMIC vaddr near u64::MAX
    let mut bad = img2.bytes.clone();
    bad[64 + 3 * 56 + 16..64 + 3 * 56 + 24].copy_from_slice(&(u64::MAX - 8).to_le_bytes());
    assert!(load_elf(&bad, dest).is_err());
    // PT_DYNAMIC p_offset not matching the mapped load
    let mut bad = img2.bytes.clone();
    bad[64 + 3 * 56 + 8..64 + 3 * 56 + 16].copy_from_slice(&0x400u64.to_le_bytes());
    assert!(load_elf(&bad, dest).is_err());
    // PT_DYNAMIC filesz > memsz
    let mut bad = img2.bytes.clone();
    bad[64 + 3 * 56 + 40..64 + 3 * 56 + 48].copy_from_slice(&8u64.to_le_bytes());
    assert_eq!(load_elf(&bad, dest), Err(ElfError::InvalidSegment));

    // Unsupported dynamic tags (mutate first entry's tag)
    for tag in [1i64, 17, 18, 19, 22, 23, 35, 36, 37] {
        let mut bad = img2.bytes.clone();
        bad[0x300..0x308].copy_from_slice(&tag.to_le_bytes());
        assert_eq!(
            load_elf(&bad, dest),
            Err(ElfError::Unsupported),
            "tag {tag}"
        );
    }
    // Partial RELA descriptors: drop DT_RELAENT
    {
        let mut bad = img2.bytes.clone();
        bad[0x320..0x328].copy_from_slice(&0i64.to_le_bytes()); // DT_RELAENT -> DT_NULL
        bad[0x328..0x330].copy_from_slice(&0u64.to_le_bytes());
        assert_eq!(load_elf(&bad, dest), Err(ElfError::InvalidRelocation));
    }
    assert!(dest.iter().all(|&b| b == SENTINEL));
    let _ = img;
}

#[test]
fn rejects_unterminated_dynamic() {
    let mut dyn_table = Vec::new();
    dyn_table.extend_from_slice(&7i64.to_le_bytes());
    dyn_table.extend_from_slice(&0x2040u64.to_le_bytes());
    dyn_table.extend_from_slice(&8i64.to_le_bytes());
    dyn_table.extend_from_slice(&24u64.to_le_bytes());
    // No DT_NULL before end of segment.
    let img = build(
        &[
            seg(PT_LOAD, PF_R | PF_X, 0x100, 0, 4, 4),
            seg(PT_LOAD, PF_R | PF_W, 0x300, 0x2000, 0x40, 0x40),
            seg(
                PT_DYNAMIC,
                PF_R | PF_W,
                0x300,
                0x2000,
                dyn_table.len() as u64,
                dyn_table.len() as u64,
            ),
        ],
        &[(0x100, &[0x1f, 0x20, 0x03, 0xd5]), (0x300, &dyn_table)],
        0,
    );
    let mut backing = dest_backing(0x4000);
    let dest = dest_of(&mut backing, 0x3000);
    assert_eq!(load_elf(&img.bytes, dest), Err(ElfError::InvalidRelocation));
    assert!(dest.iter().all(|&b| b == SENTINEL));
}

// ---- Real artifacts: the three user programs built by `make users` ---------

const USER_ELFS: [(&str, &[u8]); 3] = [
    ("squares", include_bytes!("../../user/squares.elf")),
    ("pi", include_bytes!("../../user/pi.elf")),
    ("primecheck", include_bytes!("../../user/primecheck.elf")),
];

// Independent read-only ELF parse used to build the expected image.
struct RealPhdr {
    p_type: u32,
    flags: u32,
    offset: u64,
    vaddr: u64,
    filesz: u64,
    memsz: u64,
    align: u64,
}

fn parse_phdrs(img: &[u8]) -> Vec<RealPhdr> {
    let phoff = u64::from_le_bytes(img[32..40].try_into().unwrap()) as usize;
    let phnum = u16::from_le_bytes(img[56..58].try_into().unwrap()) as usize;
    (0..phnum)
        .map(|i| {
            let o = phoff + 56 * i;
            RealPhdr {
                p_type: u32::from_le_bytes(img[o..o + 4].try_into().unwrap()),
                flags: u32::from_le_bytes(img[o + 4..o + 8].try_into().unwrap()),
                offset: u64::from_le_bytes(img[o + 8..o + 16].try_into().unwrap()),
                vaddr: u64::from_le_bytes(img[o + 16..o + 24].try_into().unwrap()),
                filesz: u64::from_le_bytes(img[o + 32..o + 40].try_into().unwrap()),
                memsz: u64::from_le_bytes(img[o + 40..o + 48].try_into().unwrap()),
                align: u64::from_le_bytes(img[o + 48..o + 56].try_into().unwrap()),
            }
        })
        .collect()
}

fn rd_u64(b: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}

#[test]
fn loads_real_user_elfs() {
    for (name, img) in USER_ELFS {
        let image_copy = img.to_vec();
        let phdrs = parse_phdrs(img);
        let loads: Vec<&RealPhdr> = phdrs.iter().filter(|p| p.p_type == PT_LOAD).collect();
        assert!(!loads.is_empty(), "{name}");
        let span = loads.iter().map(|p| p.vaddr + p.memsz).max().unwrap() as usize;
        let align = loads.iter().map(|p| p.align).max().unwrap().max(1);

        let alignment = usize::try_from(align).unwrap().max(16);
        assert!(alignment.is_power_of_two());
        let capacity = span.checked_add(alignment).unwrap();
        let mut backing = dest_backing(capacity);
        let raw = backing.as_mut_ptr() as usize;
        let skip = (alignment - raw % alignment) % alignment;
        assert_eq!(skip % 16, 0);
        let dest = dest_of(&mut backing[skip / 16..], span);
        let base = dest.as_ptr() as u64;

        let entry = load_elf(img, dest).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let e_entry = rd_u64(img, 24);
        assert_eq!(entry, (base + e_entry) as *const (), "{name}");

        // Independently build the expected image: sentinel everywhere, file
        // bytes copied, bss tails zeroed.
        let mut expected = vec![SENTINEL; span];
        for p in &loads {
            let (vo, fs, ms, fo) = (
                p.vaddr as usize,
                p.filesz as usize,
                p.memsz as usize,
                p.offset as usize,
            );
            expected[vo..vo + fs].copy_from_slice(&img[fo..fo + fs]);
            expected[vo + fs..vo + ms].fill(0);
        }

        // Independently apply the RELATIVE relocations from the image's own
        // dynamic table, and count them.
        let mut nfixups = 0usize;
        for p in &phdrs {
            if p.p_type != PT_DYNAMIC {
                continue;
            }
            let mut rela_v = None;
            let mut rela_sz = None;
            for i in 0..(p.filesz / 16) {
                let off = (p.offset + i * 16) as usize;
                let tag = i64::from_le_bytes(img[off..off + 8].try_into().unwrap());
                let val = rd_u64(img, off + 8);
                match tag {
                    0 => break,
                    7 => rela_v = Some(val),
                    8 => rela_sz = Some(val),
                    _ => {}
                }
            }
            if let (Some(rv), Some(rs)) = (rela_v, rela_sz) {
                // Map the rela vaddr to a file offset via the PT_LOADs.
                let foff = loads
                    .iter()
                    .find(|s| rv >= s.vaddr && rv + rs <= s.vaddr + s.filesz)
                    .map(|s| s.offset + (rv - s.vaddr))
                    .unwrap();
                for e in 0..rs / 24 {
                    let ro = (foff + e * 24) as usize;
                    let r_off = rd_u64(img, ro) as usize;
                    let r_info = rd_u64(img, ro + 8);
                    let r_add = rd_u64(img, ro + 16) as i64;
                    assert_eq!(r_info, 1027, "{name}: non-RELATIVE reloc");
                    assert!(r_add >= 0, "{name}");
                    expected[r_off..r_off + 8]
                        .copy_from_slice(&(base + r_add as u64).to_le_bytes());
                    nfixups += 1;
                }
            }
        }
        assert!(nfixups > 0, "{name}: expected at least one relative fixup");
        assert_eq!(dest, &expected[..], "{name}: loaded image mismatch");
        // The source image must be untouched (loader borrows it read-only).
        assert_eq!(img, &image_copy[..], "{name}");
        // Gaps between segments must still be sentinel (checked via expected,
        // which is all-sentinel outside loaded ranges).
    }
}
