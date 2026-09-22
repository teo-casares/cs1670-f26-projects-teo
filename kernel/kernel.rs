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

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
