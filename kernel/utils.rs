// Memory-mapped I/O helper functions for operating on 32-bit memory-mapped I/O
// "registers". `mmio_read32` reads the value of the register at `addr`, and
// `mmio_write32` writes a value to the register at `addr`.
//
// The volatile access is important because it tells the compiler that the
// value in the memory address can change at any time (since the address is
// wired to hardware device, rather than actual memory). This also prevents
// the compiler from optimizing away reads or writes to the MMIO register.
pub unsafe fn mmio_read32(addr: usize) -> u32 {
    unsafe { core::ptr::read_volatile(addr as *const u32) }
}

pub unsafe fn mmio_write32(addr: usize, v: u32) {
    unsafe { core::ptr::write_volatile(addr as *mut u32, v) }
}

// Spin the CPU for at least `ctr` cycles. Returns the number of cycles
// remaining (should be 0).
pub fn delay_cycles(mut ctr: u64) -> u32 {
    while ctr != 0 {
        unsafe { core::arch::asm!("nop", options(nomem, nostack, preserves_flags)) };
        ctr -= 1;
    }
    0
}

// Copy n bytes from memory at `src` into memory at `dest`
// (works with overlapping memory).
pub unsafe fn memmove(dest: *mut u8, src: *const u8, n: u64) -> *mut u8 {
    unsafe { core::ptr::copy(src, dest, n as usize) };
    dest
}

// Wrapper around memmove that assumes the memory regions do not overlap.
// Provides the familiar `memcpy` interface.
pub unsafe fn memcpy(dest: *mut u8, src: *const u8, n: u64) -> *mut u8 {
    unsafe { core::ptr::copy_nonoverlapping(src, dest, n as usize) };
    dest
}

// Fills the first n bytes of the memory pointed by src with the constant byte c
pub unsafe fn memset(src: *mut u8, c: i32, n: u64) -> *mut u8 {
    unsafe { core::ptr::write_bytes(src, c as u8, n as usize) };
    src
}
