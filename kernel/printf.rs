use crate::drivers::uart::uart_send;
use core::ffi::{CStr, VaList, c_char, c_int, c_long, c_uint, c_ulong, c_void};

pub type VprintfFn = unsafe extern "C" fn(*const c_char, VaList<'_>) -> c_int;

fn send_bytes(bytes: &[u8]) -> c_int {
    for &byte in bytes {
        uart_send(byte);
    }
    bytes.len().min(c_int::MAX as usize) as c_int
}

fn send_number(mut number: u64, radix: u64, width: usize) -> c_int {
    let mut digits = [b'0'; 20];
    let mut count = 0;
    loop {
        digits[count] = b"0123456789abcdef"[(number % radix) as usize];
        count += 1;
        number /= radix;
        if number == 0 {
            break;
        }
    }
    let length = count.max(width);
    for _ in count..length {
        uart_send(b'0');
    }
    while count > 0 {
        count -= 1;
        uart_send(digits[count]);
    }
    length as c_int
}

fn send_signed(number: i64) -> c_int {
    let sign = if number < 0 { send_bytes(b"-") } else { 0 };
    sign + send_number(number.unsigned_abs(), 10, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vprintf(fmt: *const c_char, mut args: VaList<'_>) -> c_int {
    if fmt.is_null() {
        return 0;
    }
    // fmt raw pointer
    // scan for terminating zero and return borrowed &Cstr
    // unsafe because rust can't know that fmt points to readable memory
    let bytes = unsafe { CStr::from_ptr(fmt) }.to_bytes();
    let mut written: c_int = 0;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            uart_send(bytes[index]);
            written = written.saturating_add(1);
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        let long = bytes.get(index) == Some(&b'l');
        if long {
            index += 1;
        }
        let Some(&specifier) = bytes.get(index) else {
            written = written.saturating_add(send_bytes(&bytes[start..index]));
            break;
        };
        index += 1;
        let emitted = match (long, specifier) {
            (false, b'%') => send_bytes(b"%"),
            (false, b'c') => {
                let character = unsafe { args.next_arg::<c_int>() };
                uart_send(character as u8);
                1
            }
            (false, b's') => {
                let string_ptr = unsafe { args.next_arg::<*const c_char>() };
                let string_bytes = if string_ptr.is_null() {
                    b"(null)".as_slice()
                } else {
                    unsafe { CStr::from_ptr(string_ptr) }.to_bytes()
                };
                send_bytes(string_bytes)
            }
            (false, b'd') => send_signed(unsafe { args.next_arg::<c_int>() } as i64),
            (true, b'd') => send_signed(unsafe { args.next_arg::<c_long>() } as i64),
            (false, b'u') => send_number(unsafe { args.next_arg::<c_uint>() } as u64, 10, 0),
            (true, b'u') => send_number(unsafe { args.next_arg::<c_ulong>() } as u64, 10, 0),
            (false, b'x') => send_number(unsafe { args.next_arg::<c_uint>() } as u64, 16, 8),
            (true, b'x') => send_number(unsafe { args.next_arg::<c_ulong>() } as u64, 16, 16),
            (false, b'p') => {
                let pointer = unsafe { args.next_arg::<*const c_void>() };
                send_bytes(b"0x")
                    + send_number(
                        pointer as usize as u64,
                        16,
                        core::mem::size_of::<usize>() * 2,
                    )
            }
            _ => send_bytes(&bytes[start..index]),
        };
        written = written.saturating_add(emitted);
    }
    written
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printf(fmt: *const c_char, args: ...) -> c_int {
    unsafe { vprintf(fmt, args) }
}
