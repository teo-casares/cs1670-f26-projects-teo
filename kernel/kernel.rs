#![no_std]
#![no_main]
#![no_builtins]

use core::arch::global_asm;

mod drivers;
mod elf;
mod init;
mod memlayout;
mod mm;
mod pi;
mod printf;
mod utils;

// Make INITIAL_KERNEL_STACK usable from assembly (boot.S) without requiring
// students to touch the Rust wiring.
global_asm!(
    ".equ INITIAL_KERNEL_STACK, {stack}",
    stack = const memlayout::INITIAL_KERNEL_STACK
);
global_asm!(include_str!("boot.S"), options(raw));
global_asm!(".section .text", include_str!("mm.S"), options(raw));

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    drivers::uart::uart_init();
    unsafe {
        core::ptr::write_volatile(
            memlayout::F_VPRINTF as *mut usize,
            printf::vprintf as *const () as usize,
        );

        printf::printf(c"Hello world from FerrOS!\r\n".as_ptr());
        match init::init() {
            Ok(()) => {}
            Err(_error) => {
                printf::printf(c"Failed to load squares. \r\n".as_ptr());
            }
        }
    }

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
