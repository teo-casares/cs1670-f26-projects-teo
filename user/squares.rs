#![no_std]
#![no_main]
#![no_builtins]

/* squares.rs - Compute squares of integers from 1 to n and their differences
 *
 * Following in the tradition of a program run on many early computers to test
 * that their arithmetic units were working correctly, this program computes the
 * squares of integers from 1 to n and prints each integer, its square, and the
 * difference between the current square and the previous square. The output is
 * formatted in a tabular form.
 */

use core::ffi::c_int;

use u_common::{exit, printf};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let n: c_int = 100; // Number of squares to compute
    let mut p: c_int = 0; // Previous square value

    for i in 1..=n {
        let s = i * i; // Compute the square of i
        // Print i, square, and difference from previous square
        unsafe { printf(c"%d\t%d\t%d\r\n".as_ptr(), i, s, s - p) };
        p = s; // Update previous square value
    }

    exit();
    0
}
