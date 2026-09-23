#![allow(clippy::identity_op)]

use crate::memlayout::PERIPHERALS_BASE;
use crate::drivers::gpio::gpio_init;
use crate::utils::{mmio_write32, mmio_read32};

// PL011 UART (p. 175ff, §13; note errata)



pub const UART_BASE: usize = PERIPHERALS_BASE + 0x0020_1000;
pub const UART_DR: usize = UART_BASE + 0x0; // data register | does not change
pub const UART_FR: usize = UART_BASE + 0x18; // flag register
pub const UART_IBRD: usize = UART_BASE + 0x24; // Interger baud rate division
pub const UART_FBRD: usize = UART_BASE + 0x28;  // fractional baud rate divison
pub const UART_LCRH: usize = UART_BASE + 0x2C; // line control register
pub const UART_CR: usize = UART_BASE + 0x30; // Control register

pub fn uart_init() {
    gpio_init();
    unsafe {
        mmio_write32(UART_CR, 0);
        mmio_write32(UART_IBRD, 26);
        mmio_write32(UART_FBRD, 3);
        mmio_write32(UART_LCRH, (0b11 << 5) | (1 << 4));
        mmio_write32(UART_CR, (1 << 0) | (1 << 8) | (1 << 9) | ( 1 << 15));
        // 0: Uart, 8: transmitter, 9: reciever, 15: cts hardware flow control, require
    }
}

pub fn uart_send(c: u8) {
    while unsafe { mmio_read32(UART_FR)} & (1 << 5) != 0 {
        core::hint::spin_loop();
    }
    unsafe {
        mmio_write32(UART_DR, u32::from(c));
    }
}

pub fn uart_send_string(s: &core::ffi::CStr) {
    for byte in s.to_bytes() {
        uart_send(*byte)
    }
}
