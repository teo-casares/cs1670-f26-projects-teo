// Host-side tests for the Rust port of the supplied pi.c spigot program.
// kernel/pi.rs is included directly; the UART driver and variadic printf are
// stubbed (the printf parser is a student exercise).

mod drivers {
    pub mod uart {
        pub fn uart_init() {}
        pub fn uart_send(_c: u8) {}
        pub fn uart_send_string(_s: &core::ffi::CStr) {}
    }
}

mod printf {
    use core::ffi::{VaList, c_char, c_int};
    pub unsafe extern "C" fn vprintf(_fmt: *const c_char, _args: VaList<'_>) -> c_int {
        0
    }
    pub unsafe extern "C" fn printf(_fmt: *const c_char, _args: ...) -> c_int {
        0
    }
}

#[path = "../../kernel/pi.rs"]
mod pi;

// compute_pi mutates shared file-scope statics; serialize the tests that run it.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn bcd_len_is_1001() {
    assert_eq!(pi::BCD_LEN, 1001);
    assert_eq!(core::mem::size_of::<[u8; pi::BCD_LEN]>(), 1001);
}

#[test]
fn compute_pi_digits() {
    // Frozen oracle: tests/pi_bcd.txt was generated once from the original
    // supplied C program (user/pi.c), not from the Rust port under test.
    const EXPECTED: &[u8] = include_bytes!("../../tests/pi_bcd.txt");
    let _g = LOCK.lock().unwrap();
    unsafe { pi::compute_pi() };
    let digits = unsafe { &*core::ptr::addr_of!(pi::pi_bcd) };
    assert_eq!(digits.len(), 1001);
    assert!(digits.iter().all(|&d| d <= 9));
    let got: Vec<u8> = digits.iter().map(|d| d + b'0').collect();
    assert_eq!(got, EXPECTED.trim_ascii_end());
}

#[test]
fn compute_pi_is_repeatable() {
    let _g = LOCK.lock().unwrap();
    unsafe { pi::compute_pi() };
    let first = unsafe { *core::ptr::addr_of!(pi::pi_bcd) };
    unsafe { pi::compute_pi() };
    let second = unsafe { *core::ptr::addr_of!(pi::pi_bcd) };
    assert_eq!(first, second);
}
