#![allow(clippy::identity_op)]

use crate::memlayout::PERIPHERALS_BASE;

// PL011 UART (p. 175ff, §13; note errata)
//
// TODO: You'll need to change all offsets below from 0x0 to their true
// value (0x0 is a placeholder). We also only give you the UART_BASE
// constant and one example register (UART_DR). You will need to fill in
// the entries for the necessary registers and their offsets based on the
// information in the BCM2835 ARM Peripherals datasheet or the PL011 UART
// datasheet.
pub const UART_BASE: usize = PERIPHERALS_BASE + 0x0;
pub const UART_DR: usize = UART_BASE + 0x0; // data register
// pub const UART_XXX: usize = UART_BASE + TODO;    // ... other registers

pub fn uart_init() {
    todo!("Quest 2: initialize UART")
}

pub fn uart_send(_c: u8) {
    todo!("Quest 2: send a byte")
}

pub fn uart_send_string(_s: &core::ffi::CStr) {
    todo!("Quest 2: send a string")
}
