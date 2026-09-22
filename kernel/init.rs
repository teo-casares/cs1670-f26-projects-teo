// User program images, linked into the kernel via the .elf_executables
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
