// The `+ 0x0` entries below are intentional placeholders, mirroring the
// supplied C header, to be filled in during the quests.
#![allow(clippy::identity_op)]

// Raspberry Pi 3B (BCM2837 SoC) physical memory layout,
// as defined in official documentation:
// https://www.raspberrypi.org/app/uploads/2012/02/BCM2835-ARM-Peripherals.pdf
//
// except for the following differences:
// - RPi 3 uses a peripheral base address of 0x3F00'0000 rather than
//   0x2000'0000

// Location of the initial kernel stack in physical memory.
// Must be 16-byte aligned.
// pub const INITIAL_KERNEL_STACK: usize = YOUR_CHOSEN_VALUE;
pub const INITIAL_KERNEL_STACK: usize = 0;

// Peripherals are mapped starting at 0x3F00'0000, giving 1 MB for I/O memory
// Note that the documentation lists addresses starting at 0x7E00'0000. This is
// where the actual ARM hardware has these peripherals, but they are mapped into
// the SoC's address space at 0x3F00'0000
pub const PERIPHERALS_BASE: usize = 0x3F000000;
pub const PERIPHERALS_LIMIT: usize = 0x3FFFFFFF;

// Trampoline to user-reachable functions (like yield or printf).
// This is essentially an array of function pointers at a known location, so
// that programs can use these functions as a library.
//
// The pointers get initialized in `kernel_main` (kernel.rs) to point to the
// actual machine code. User programs can then call kernel functions by simply
// following those pointers (even though they weren't compiled together with
// the kernel).
// TODO: you'll need to change F_BASE in Quest 3 of Project 1.
pub const F_BASE: usize = 0;
pub const F_VPRINTF: usize = F_BASE.wrapping_sub(core::mem::size_of::<usize>());
