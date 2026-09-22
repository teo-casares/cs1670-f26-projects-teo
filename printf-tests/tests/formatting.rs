#![no_builtins]

// Hosted test for the student-implemented kernel printf (kernel/printf.rs).
// The UART "driver" below is a capture-only test seam — it is not an MMIO
// driver and says nothing about real hardware behavior.
//
// This test is EXPECTED TO FAIL on the untouched scaffold: `vprintf` is a
// `todo!()` stub until Quest 2 is implemented. Hosted tests also do not
// establish the AArch64 variadic ABI or hardware behavior; the real user
// programs still need to run on target.

use core::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void};

mod drivers {
    pub mod uart {
        use std::sync::Mutex;
        pub static OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
        pub fn uart_init() {}
        pub fn uart_send(c: u8) {
            OUTPUT.lock().unwrap().push(c);
        }
        pub fn uart_send_string(s: &core::ffi::CStr) {
            for &c in s.to_bytes() {
                uart_send(c);
            }
        }
    }
}

#[path = "../../kernel/printf.rs"]
mod printf;

fn reset() {
    drivers::uart::OUTPUT.lock().unwrap().clear();
}

fn take() -> Vec<u8> {
    std::mem::take(&mut *drivers::uart::OUTPUT.lock().unwrap())
}

unsafe extern "C" fn via_vprintf(fmt: *const c_char, args: ...) -> c_int {
    unsafe { crate::printf::vprintf(fmt, args) }
}

#[test]
fn printf_formatting() {
    reset();
    unsafe { printf::printf(c"plain %% text\r\n".as_ptr()) };
    assert_eq!(take(), b"plain % text\r\n");

    reset();
    unsafe {
        printf::printf(
            c"%d %d %d %u".as_ptr(),
            0 as c_int,
            -1 as c_int,
            c_int::MIN,
            c_uint::MAX,
        )
    };
    assert_eq!(take(), b"0 -1 -2147483648 4294967295");

    reset();
    unsafe {
        printf::printf(
            c"%ld %ld %lu".as_ptr(),
            c_long::MIN,
            c_long::MAX,
            c_ulong::MAX,
        )
    };
    assert_eq!(
        String::from_utf8(take()).unwrap(),
        format!("{} {} {}", c_long::MIN, c_long::MAX, c_ulong::MAX)
    );

    reset();
    unsafe { printf::printf(c"%c %s".as_ptr(), b'A' as c_int, c"hello".as_ptr()) };
    assert_eq!(take(), b"A hello");

    reset();
    unsafe { printf::printf(c"%x %lx".as_ptr(), 0x12 as c_uint, 0x34 as c_ulong) };
    assert_eq!(take(), b"00000012 0000000000000034");

    reset();
    unsafe { printf::printf(c"%p".as_ptr(), 0x1234usize as *const c_void) };
    let bytes = take();
    let text = std::str::from_utf8(&bytes).unwrap();
    let digits = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    assert_eq!(u64::from_str_radix(digits, 16).unwrap(), 0x1234);

    reset();
    unsafe {
        printf::printf(
            c"%d,%d,%d,%d,%d,%d,%d,%d,%d,%d".as_ptr(),
            1 as c_int,
            2 as c_int,
            3 as c_int,
            4 as c_int,
            5 as c_int,
            6 as c_int,
            7 as c_int,
            8 as c_int,
            9 as c_int,
            10 as c_int,
        )
    };
    assert_eq!(take(), b"1,2,3,4,5,6,7,8,9,10");

    // vprintf bridge: variadic args must forward correctly.
    reset();
    unsafe { via_vprintf(c"%d %s".as_ptr(), -42 as c_int, c"forwarded".as_ptr()) };
    assert_eq!(take(), b"-42 forwarded");

    // Malformed formats: only require that the call returns.
    for fmt in [c"%", c"%l", c"%q"] {
        unsafe { printf::printf(fmt.as_ptr()) };
    }
}
