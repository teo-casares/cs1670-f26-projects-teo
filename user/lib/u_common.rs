#![no_std]
#![no_builtins]
// The F_BASE asserts are placeholder guards for Quest 3.
#![allow(clippy::absurd_extreme_comparisons, clippy::manual_is_multiple_of)]

#[path = "../../kernel/memlayout.rs"]
mod memlayout;

use core::ffi::{VaList, c_char, c_int};

pub type VprintfFn = unsafe extern "C" fn(*const c_char, VaList<'_>) -> c_int;

// calls the function which address is contained in F_VPRINTF
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printf(fmt: *const c_char, args: ...) -> c_int {
    unsafe {
        assert!(memlayout::F_BASE >= core::mem::size_of::<usize>());
        assert!(memlayout::F_BASE % 8 == 0);
        let f: VprintfFn = core::mem::transmute(core::ptr::read_volatile(
            memlayout::F_VPRINTF as *const usize,
        ));
        f(fmt, args)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn exit() {
    // Returning from `main` ends the program; the kernel reaps it there.
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
