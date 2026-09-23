use crate::elf::{ElfError, load_elf};
use crate::memlayout::{PROC_SIZE, PROC_START};

// User program images, linked into the kernel via the .elf_executables
//
// section (see kernel/linker.ld). We need this section to ensure programs
// are loaded into memory until we have disk I/O and a file system.

#[used]
#[unsafe(link_section = ".elf_executables")]
pub static SQUARES: [u8; include_bytes!("../user/squares.elf").len()] =
    *include_bytes!("../user/squares.elf");

#[used]
#[unsafe(link_section = ".elf_executables")]
pub static PI: [u8; include_bytes!("../user/pi.elf").len()] = *include_bytes!("../user/pi.elf");

#[used]
#[unsafe(link_section = ".elf_executables")]
pub static PRIMECHECK: [u8; include_bytes!("../user/primecheck.elf").len()] =
    *include_bytes!("../user/primecheck.elf");

pub unsafe fn init() -> Result<(), ElfError> {
    let base = PROC_START as *mut u8;
    let images: [&[u8]; 3] = [&SQUARES, &PI, &PRIMECHECK];
    unsafe { core::ptr::write_bytes(base, 0, PROC_SIZE) };
    for image in images {
        let entry = {
            let dest = unsafe { core::slice::from_raw_parts_mut(base, PROC_SIZE)};
            load_elf(image, dest)?
        };
        unsafe {
            let program: unsafe extern "C" fn() -> core::ffi::c_int =
                core::mem::transmute(entry);
            program();
        }
    }
    Ok(())
}
