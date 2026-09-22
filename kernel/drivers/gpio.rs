#![allow(clippy::identity_op, clippy::assertions_on_constants)]

use crate::memlayout::PERIPHERALS_BASE;
use crate::utils::{delay_cycles, mmio_read32, mmio_write32};

// GPIO pin configuration registers (p. 90ff, §6.1)
//
// TODO: all of these are set to PERIPHERALS_BASE + 0x0; you will need to
// adjust this with the correct offsets determined from the BCM2835 ARM
// Peripherals datasheet.

// GPIO Function Select Registers (GPFSELn): these select what function is
// mapped to each GPIO pin.
// See table on p. 102 and p. 91ff of datasheet.
pub const GPFSEL0: usize = PERIPHERALS_BASE + 0x0; // pins 0-9
pub const GPFSEL1: usize = PERIPHERALS_BASE + 0x0; // pins 10-19

// GPIO Pin Up/Down Enable Register (GPPUD): this controls whether pins are
// enabled for "pull-up" or "pull-down", which sets the default voltage level
// of the pin.
// See p. 90, §6.1 of datasheet. Note that addresses starting with 0x7E...
// need translating to 0x3F... to be accessible to the CPU.
// Register documentation for GPUUD on p. 100f, §6.1.
pub const GPPUD: usize = PERIPHERALS_BASE + 0x0;
// GPIO Pin Up/Down Enable Clock Registers (GPPUDCLKn): these control the
// actually pull-up/pull-down state of the pins, based on the value set in
// GPPUD. Make sure to read the text on p. 101 (§6.1) of the datasheet to
// understand how to use GPUUD and GPPUDCLKn together correctly.
pub const GPPUDCLK0: usize = PERIPHERALS_BASE + 0x0;

// Configure and enable the GPIO pins for the PL011 UART device.
pub fn gpio_init() {
    // Refuse to touch MMIO while the register offsets are still placeholders.
    assert!(GPFSEL1 != PERIPHERALS_BASE);
    assert!(GPPUD != PERIPHERALS_BASE);
    assert!(GPPUDCLK0 != PERIPHERALS_BASE);

    // Select the right alternative function for each pin we're using
    let mut selector = unsafe { mmio_read32(GPFSEL1) };
    selector &= !(0b111 << 12); // clear bits for pin 14
    selector |= 0b100 << 12; // set pin 14 to ALT0 functionality (TXD0)
    selector &= !(0b111 << 15); // clear bits for pin 15
    selector |= 0b100 << 15; // set pin 15 to ALT0 functionality (RXD0)
    selector &= !(0b111 << 18); // clear bits for pin 16
    selector |= 0b111 << 18; // set pin 16 to ALT3 functionality (CTS0)
    unsafe { mmio_write32(GPFSEL1, selector) };

    // Enable the GPIO pins
    unsafe { mmio_write32(GPPUD, 0) }; // disable pull-up/down for pins 14, 15, and 16
    delay_cycles(150);
    // enable clock for pins 14, 15, and 16; a clock signal is necessary so that
    // the configuration change actually gets applied
    unsafe { mmio_write32(GPPUDCLK0, (0b1 << 14) | (0b1 << 15) | (0b1 << 16)) };
    delay_cycles(150);
    unsafe { mmio_write32(GPPUDCLK0, 0) }; // disable clock again
}
