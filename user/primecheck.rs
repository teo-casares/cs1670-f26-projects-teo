#![no_std]
#![no_main]
#![no_builtins]

use core::ffi::c_int;

use u_common::{exit, printf};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let limit: u32 = 2 << 27; // ~134M
    let mut prime_count: u32 = 0;

    for i in 2..limit {
        let mut prime = true;
        for j in 2..i {
            if i % j == 0 {
                // not a prime, try next i
                prime = false;
                break;
            }
        }
        if prime {
            prime_count += 1;
            if prime_count % 1000 == 0 {
                unsafe {
                    printf(
                        c"primecheck: Found another 1000 primes; last one was %d!\r\n".as_ptr(),
                        i as c_int,
                    )
                };
            }
        }
    }
    exit();
    0
}
