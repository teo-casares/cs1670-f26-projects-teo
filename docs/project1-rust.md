# Project 1 (Rust): Booting Your Own Operating System

*Translated handout for the Rust scaffold. The canonical assignment is the
course page: <https://csci1670.github.io/docs/project1/>. This document is a
derivative of that handout (© Brown University CS 1670 course staff), used
under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). Where this
translation and the course page disagree about grading or policy, the course
page wins. Source links are collected in the [appendix](#sources).*

In this project, you will write the code that takes a Raspberry Pi from the
moment it powers on to the point where it can run a sequence of programs, one
after another. By the end, you will have built a tiny operating system that
performs *batch processing*: it loads a program into memory, runs it to
completion, and then loads and runs the next one — the way the very first
operating systems worked in the 1950s and 60s.

You will start with nothing. There is no operating system underneath you, no
standard library, no `printf`, just you and the hardware. This is called
"bare-metal" programming, and in Rust it means `#![no_std]`: no `std`, no
heap, no threads, no OS facilities at all. (Note that `#![no_std]` does not
by itself forbid `alloc`; this project simply chooses to have no allocator.)

Two jobs of an OS, both of which you will implement in simple form:

- **Abstraction.** A program should not have to know that the serial
  console/UART device lives at a particular physical address, or how to
  configure it, or how its memory segments are laid out. The OS hides that
  behind clean interfaces (e.g., a `printf` the program just calls).
- **Mediation.** Hardware is shared between programs, but sharing must be
  orchestrated: only one program can use a CPU at a time, so something must
  decide which program runs and when. In this project your OS is the
  mediator: it picks the next program, loads it, runs it to completion, and
  picks another.

## CS 1670 vs. 1690/2670

| CS 1670 | CS 1690/2670 |
|---|---|
| Complete quests 1–3 in QEMU (emulated Raspberry Pi). | Complete quests 1–3 in QEMU **and** run your OS on real hardware (Raspberry Pi 3B/3B+ — **not** Pi 4 or 5). |

Real hardware often unearths bugs that do not appear in QEMU — QEMU does not
emulate the GPIO pin configuration at all, for example. If you are in CS
1690/2670, test on real hardware as early as possible: on your own Pi, via
the grading server's remote Pi, or at instructor office hours. See
[Extra requirements for CS 1690/2670](#extra-requirements-for-cs-16902670-students).

## Roadmap

| Task | What you build | What you learn |
|---|---|---|
| 1. Boot | Assembly bootup code (`kernel/boot.S`) that starts a bare-metal program that estimates π | The boot process: choosing a CPU, the stack, zeroing globals, jumping into code |
| 2. Output | A driver for the ARM PL011 UART and a `printf` implementation, used to print π | Memory-mapped I/O and device drivers |
| 3. Run programs | A minimal kernel (`kernel/kernel.rs`) that loads ELF executables and runs several programs in sequence, sharing `printf` across them | Executables vs. processes, ELF loading, the OS as abstraction and mediator |

## Getting started

### Toolchain

The build is pinned by `rust-toolchain.toml` to `beta-2026-09-20` with target
`aarch64-unknown-none-softfloat` and components `rustfmt`, `clippy`,
`llvm-tools`. Install once on the host:

```sh
rustup toolchain install beta-2026-09-20 --profile minimal \
  -c rustfmt -c clippy -c llvm-tools -t aarch64-unknown-none-softfloat
```

You also need `qemu-system-aarch64`, `make`, `python3`, and GDB (plain `gdb`
if it has AArch64 support, `gdb-multiarch` otherwise).

You can work **on the host or inside the course container** — one shared
checkout is visible at different paths, with separate toolchain installs:

- host: `/home/teo/cs1670/container-home/projects`
- inside `cs1670-container` (as `cs1670-user`): `~/projects`

The base course image ships without Rust; in this setup rustup is installed
in the container user's bind-mounted home (`~/.cargo`, `~/.rustup`), so it
survives container restarts. An already-open container shell needs
`source "$HOME/.cargo/env"` once; new login shells get it from `~/.bashrc`.
On a fresh container/home, install rustup per <https://rustup.rs> and then
the pinned toolchain command above — see `README.md` for details.

Quick start on this machine (adjust the path on your own host):

```sh
cd /home/teo/cs1670/container-home/projects   # or: cd ~/projects in the container
rustup toolchain install beta-2026-09-20 --profile minimal \
  -c rustfmt -c clippy -c llvm-tools -t aarch64-unknown-none-softfloat
make users      # build user/*.elf first — required before kernel/test builds
make check      # cargo check --workspace
make test       # host-side scaffold tests
```

Open your editor at this `projects` directory (the Cargo workspace root).
Note that **Make orchestrates the artifacts**: `cargo build` alone does not
produce or refresh `user/*.elf`, `kernel8.img`, or `armstub.bin` — always go
through `make users` / the `make` targets below, and re-run `make users`
after editing `kernel/memlayout.rs` (the user crate shares that file, and
its output — including relocation contents — changes with it).

> `.cargo/config.toml` sets the default target and
> `-C target-cpu=cortex-a53 -C relocation-model=pic`. The kernel is linked
> fixed at `0x80000` (via `kernel/linker.ld`); user programs are linked as
> zero-based PIE executables (via `user/procs.ld`).

### File map (C name → Rust name)

| Upstream C | Rust scaffold | Notes |
|---|---|---|
| `kernel/kernel.c` | `kernel/kernel.rs` | Crate root (`#![no_std]` `#![no_main]` `#![no_builtins]`); `kernel_main` is yours to write |
| `kernel/boot.S` | `kernel/boot.S` | Unchanged language: assembly, included via `global_asm!` |
| `kernel/mm.S` | `kernel/mm.S` | Provided `memzero` helper |
| `kernel/mm.h` | `kernel/mm.rs` | `unsafe extern "C" { fn memzero(...) }` declaration |
| `kernel/memlayout.h` | `kernel/memlayout.rs` | Shared by kernel *and* user crate (single source via `#[path]`) |
| `kernel/types.h` | `core::*` | `u8..u64`, `i8..i64`, `usize`, `core::ffi::VaList` replace typedefs/builtins |
| `kernel/utils.{c,h}` | `kernel/utils.rs` | `mmio_read32`/`mmio_write32` are `unsafe fn`; `delay_cycles`; `core::ptr` wrappers replace the buggy C `memmove`/`memcpy`/`memset` |
| `kernel/drivers/uart.{c,h}` | `kernel/drivers/uart.rs` | Register constants still placeholders; functions are `todo!()` stubs |
| `kernel/drivers/gpio.h` + given `gpio_init` | `kernel/drivers/gpio.rs` | Provided port of the *supplied* GPIO code |
| `kernel/printf.{c,h}` | `kernel/printf.rs` | `vprintf`/`printf` signatures and ABI provided; the parser is your exercise |
| `kernel/pi.c` | `kernel/pi.rs` | Provided port (`pi_main`, `pi_bcd`, `compute_pi`) |
| `kernel/elf.{c,h}` | `kernel/elf.rs` | **Provided**: a bounded ELF64 loader (see below) — not the C loader |
| `kernel/init.c` | `kernel/init.rs` | Provided: embeds `user/*.elf` into `.elf_executables`; the `init` function itself is yours |
| `kernel/linker.ld` | `kernel/linker.ld` | Same layout, plus `KEEP(.text.boot)`, wildcard Rust sections, an 8-byte-aligned `bss` end, and a link-time `ASSERT(DEFINED(_start))` |
| `user/*.c` | `user/{squares,pi,primecheck}.rs` | Provided ports; built as `user/*.elf` |
| `user/lib/u_common.{c,h}` + `u_klib.h` | `user/lib/u_common.rs` | Provided user-side `printf` trampoline shim and `exit()` |
| `user/procs.ld` | `user/procs.ld` | `ENTRY(main)`, PIE at vaddr 0, `PT_LOAD`/`PT_DYNAMIC` program headers |
| `bootloader/armstub.S` + `-Ttext=0` | `bootloader/armstub.rs`, `armstub.S`, `linker.ld` | Provided; `armstub.bin` output is byte-identical to the upstream build |

The original `.c`/`.h` files remain in the tree as inactive references only;
nothing compiles them.

### Build and test commands

- `make users` — builds the three user programs and copies them to
  `user/*.elf`. Works now.
- `make check` — `cargo check --workspace` (no final kernel link). Works now.
- `make test` — host-side scaffold tests (ELF loader unit tests incl. loading
  the real `user/*.elf`, pi port vs. the frozen C oracle `tests/pi_bcd.txt`,
  and the Python output-oracle unit tests). Works now.
- `make test-printf` — hosted test of the real `kernel/printf.rs` against a
  capture-only mock UART. **Expected to fail** until you implement the
  formatter in Quest 2.
- `make test-qemu` — boots `kernel8.img` under QEMU and applies the batch
  output oracle to captured serial output. Needs Quest 3 done.
- `make test-qemu-pi` — same harness, pi-output mode. A serial-output check:
  it needs Quest 2 done (it is **not** a scripted replacement for the
  Quest 1 GDB checks).
- `make test-log LOG=<file> [MODE=batch|pi]` — applies the same oracle to a
  log you captured yourself (QEMU or serial). It only reads the file; it does
  **not** prove the log came from real hardware, validate UART framing, or
  attest registers/electrical setup.
- `make inspect-users` — dumps ELF file/program headers of `user/*.elf`
  (entry point, `PT_LOAD`/`PT_DYNAMIC`, `p_vaddr`, `p_memsz`, `p_align`).
  Works now — use it to size your program region in Quest 3.
- `make inspect-kernel` — dumps the linked kernel ELF's headers, sections,
  and sorted symbols (`bss_begin`/`bss_end`, `elf_executables_end`, …).
  Needs Quest 1 (it links `kernel8.img` first).
- `make test-suite` — `test` + `test-printf` + `test-qemu`.
- `make kernel8.img` — links the kernel and produces the image. **Expected to
  fail** with `Quest 1: implement _start in kernel/boot.S` until Quest 1.
- `make armstub.bin` — builds the bootloader blob. Works now.
- `make format`, `make clean`, `make qemu`, `make qemu-gdb`, `make help`.

Nothing in this scaffold can boot yet: `_start`, the UART driver, the printf
parser, `kernel_main`, and the program-loading `init` are all deliberately
missing. No test can pass "end-to-end" on the untouched scaffold.

### What works when

| Stage | Command | Expected outcome |
|---|---|---|
| Now (untouched scaffold) | `make check`, `make test`, `make armstub.bin`, `make inspect-users` | **PASS** |
| Now | `make kernel8.img`, `make inspect-kernel` | **FAIL**: `Quest 1: implement _start in kernel/boot.S` — intentional |
| Now | `make test-printf` | **FAIL** at the `vprintf` `todo!()` — intentional until Quest 2 |
| After Q1 | `make kernel8.img`, `make inspect-kernel`, `make qemu`, `make qemu-gdb` | Link + boot to your `pi_main`; verify in GDB |
| After Q2 | `make test-printf`, `make test-qemu-pi`, `make test-log LOG=path/to/capture.log MODE=pi` | printf tests pass; pi digits over serial (Quest 2 check — *not* a substitute for the Q1 GDB checks) |
| After Q3 | `make test-qemu`, `make test-log LOG=path/to/capture.log MODE=batch` | Batch oracle: greeting, ordered programs, 100 squares, 1,001 digits, `7919` |
| On real Pi (1690/2670) | `make all` produces `kernel8.img` + `armstub.bin`, then the manual SD/UART procedure below | Needs Q1–Q3 done **and** real hardware validation — see the hardware checklist |

Expected noise: ~57 `dead_code` warnings from `cargo check`/`build` are
normal on the scaffold — helpers are unused until your code calls them. Do
not delete helpers to silence the warnings.

QEMU notes: quit with `Ctrl-a` then `x` (pressed in sequence); `Ctrl-a`
then `c` toggles the QEMU monitor. `make kill` kills *all* of your user's
QEMU processes — course-provided escape hatch, use only if a QEMU is stuck;
it is not part of the normal flow. The GDB stub listens on port 1234 and
must be reached from the same environment where QEMU runs (both terminals
inside the container, or both on the host). The GDB connection is a debug
transport, not UART access — real-Pi output goes over the serial cable.

## Background

Read the course page's *Background* section — it is largely language-neutral:

- *What happens when a computer turns on* — the armstub (`bootloader/`) runs
  at address `0x0` in EL3 and hands over to the kernel at `0x80000`.
- *Executables vs. processes / ELF*: an ELF file is a compact description of
  a program: headers plus **program segments** (`PT_LOAD`) saying which file
  bytes go at which virtual address and how much zero-initialized memory
  (`bss`) follows them. Loading = copying segments to the addresses the OS
  chose, zeroing the tails, applying relocations, and jumping to `e_entry`.

### Rust-specific background

- **No `std`.** Kernel and user crates are `#![no_std]`; only `core` is
  available. No allocator, no `String`/`Vec`, no `println!`.
- **`unsafe` and raw pointers.** MMIO registers, linker symbols, and the
  `static mut` globals in `pi.rs` are reached through raw pointers
  (`*const`/`*mut`) and `core::ptr::addr_of_mut!`. Rust 2024 denies taking
  *references* to `static mut` by default; references you do create must
  still obey aliasing/lifetime rules even inside `unsafe`. The provided code
  prefers raw pointers — but raw pointers do not by themselves make
  concurrent access safe: rely on the fact that exactly one core runs this
  code and that globals are accessed exclusively.
- **`static mut`.** `pi_bcd`, `spigot_state` are `#[unsafe(no_mangle)]
  pub static mut` so GDB sees the same symbol names as the C version.
- **Assembly inclusion.** `kernel/kernel.rs` uses
  `core::arch::global_asm!(include_str!("boot.S"), options(raw))`, so your
  `boot.S` is assembled verbatim — same instructions, same rules as the C
  scaffold, **but** there is no C preprocessor and no `#include`. Constants
  you need from Rust are exported with
  `global_asm!(".equ INITIAL_KERNEL_STACK, {stack}", ...)`, so you can write
  `ldr x0, =INITIAL_KERNEL_STACK` in `boot.S` exactly like the C scaffold.
  There is no DWARF line info for included assembly: in GDB use `b _start`,
  `si`, and `layout asm` rather than `b file:line`.
- **Panics.** No unwinding; the provided `#[panic_handler]` spins forever.
  Break on `rust_begin_unwind` in GDB to catch panics.
- **MMIO helpers.** `mmio_read32`/`mmio_write32` do 32-bit
  `read_volatile`/`write_volatile` accesses. Volatile makes device accesses
  observable to the compiler; it is not an atomic operation, a CPU memory
  barrier, or a substitute for synchronization. MMIO reads can have
  hardware side effects.
- **`no_builtins`.** Kernel and user code build with `#![no_builtins]` (the
  analogue of the C build's `-ffreestanding`), so LLVM cannot rewrite
  `printf("%c", …)` into `putchar`/`puts` calls that don't exist.
- **Formatting numbers** without `std`: `write!`/`core::fmt` works in
  `no_std`, but you are expected to build the formatter on `uart_send` as in
  the C assignment.

## Quest 1: Boot into a bare-metal program

**Files:** `kernel/boot.S`, `kernel/memlayout.rs`, `kernel/linker.ld`,
`kernel/mm.S`, `kernel/pi.rs`.

### Memory layout diagram

Same task as upstream: create a memory layout diagram (iPad, computer, or
paper) starting from the hardware-given split between RAM (`0x0`–
`0x3EFF'FFFF`) and device/MMIO space (`0x3F00'0000`–`0x3FFF'FFFF`). You will
hand in `docs/memlayout-physical.pdf`. Update it as you pick
`INITIAL_KERNEL_STACK` (Quest 1), `F_BASE` (Quest 3), and program load
addresses.

A blank editable canvas is provided at
[`memlayout-physical.svg`](memlayout-physical.svg). Open it in Inkscape,
draw your own diagram, and save the SVG as your working source. When the
final diagram is complete, use **Save a Copy** and select PDF to create
`docs/memlayout-physical.pdf`; keep the SVG for later edits. The blank SVG
is not the finished submission.

> **AI use/coding: NOT allowed** — the point is to develop a mental image
> of your OS's memory layout.

### Bootup assembly code

`kernel/boot.S` currently contains only:

```asm
    .section ".text.boot"
    .globl _start
```

Write `_start:` code that does, in order:

1. **Park all but one CPU core.** Read `MPIDR_EL1`; the low 8 bits identify
   the core. Core 0 continues; others branch-to-self forever.
2. **Zero `.bss`.** `bss_begin`/`bss_end` come from `linker.ld`; take their
   addresses with `adr` and call the provided `memzero` (`kernel/mm.S`).
   Rust, like C, requires zero-initialized globals — nothing has done this
   for you.
3. **Set up the stack.** `mov sp` to `INITIAL_KERNEL_STACK`
   (`kernel/memlayout.rs`, 16-byte aligned, your choice of location —
   far from the kernel at `0x80000` and not overlapping devices).
   `ldr x0, =INITIAL_KERNEL_STACK` works via the `.equ` the crate emits.
4. **Jump into code.** `bl pi_main` (`kernel/pi.rs`, exported
   `unsafe extern "C" fn pi_main() -> c_int`, returns 0 like the C version;
   what happens *after* `pi_main` returns is up to your boot code's control
   flow).

Hints: keep `boot.S` tiny (~15 instructions); read `mm.S` for idioms; the
linker keeps `.text.boot` first at `0x80000` via `KEEP(*(.text.boot))`.

> **AI coding: NOT allowed** for the boot code itself (asking AI to
> *explain* ARM64 instructions is encouraged).

### Check your work (Quest 1)

- `make kernel8.img` must link (the `ASSERT(DEFINED(_start))` in
  `linker.ld` is your first milestone — until then it prints
  `Quest 1: implement _start in kernel/boot.S`).
- `make qemu-gdb`, then `gdb -nx -x .gdbinit` in a second terminal (the
  `-nx -x` flags load this repo's `.gdbinit`, which connects to QEMU and
  loads the kernel symbols — no global GDB config edits needed). Inside the
  course container use `gdb-multiarch -nx -x .gdbinit` (its plain `gdb`
  lacks AArch64 support). Set
  `b _start`, `si` through your code, break before `bl pi_main`; also
  `b rust_begin_unwind` is a useful safety net.
- Manual checklist: only core 0 does the compute (other cores parked), `sp`
  is 16-byte aligned and the stack doesn't overlap the kernel/devices,
  `.bss` is zeroed before `pi_main`, and `p pi_bcd` after `pi_main` shows
  `{3, 1, 4, 1, 5, 9, 2, 6, 5, 3, …}` — one BCD digit per byte,
  `pi_bcd[0]` = integer part.
- **Deliverable:** a GDB screenshot of the computed π digits at
  `docs/pi-gdb.png` (or `.jpg`).

## Quest 2: Print output to the console with a UART driver

**Files:** `kernel/drivers/uart.rs`, `kernel/drivers/gpio.rs`,
`kernel/memlayout.rs`, `kernel/printf.rs`, `kernel/pi.rs`.

### Part A: Find the hardware in the manual

Using the BCM2835 ARM Peripherals manual (and the community errata — the
PL011 base address in the manual is wrong), determine and fill in:

- the PL011 UART base address (`UART_BASE` in `kernel/drivers/uart.rs`),
- the offsets of the data register (`UART_DR`), flag register, baud-rate
  divisor registers (integer and fractional), line control register, and
  control register,
- the GPIO register offsets (`GPFSEL0`, `GPFSEL1`, `GPPUD`, `GPPUDCLK0` in
  `kernel/drivers/gpio.rs`) for pins 14, 15, 16.

All are `PERIPHERALS_BASE + offset`; the placeholders are `+ 0x0`. The UART
console runs at **115200 baud, 8-bit words, no parity, 1 stop bit (8N1)**,
with transmit, receive, and **CTS flow control** enabled.

> **AI use/coding: NOT allowed for the register-offset lookup** — the point
> is learning to use the hardware manual. (AI *is* allowed for the rest of
> the driver, see below.)

### Part B: Write the driver

`gpio_init()` is already provided in `kernel/drivers/gpio.rs` — a port of the
code the course page supplies (it selects the right alternate functions for
pins 14/15/16, then runs the datasheet's pull-up/down clock enable sequence).
It panics if the placeholder offsets are still `+ 0x0`, so fill in Part A
first. If you want to write it yourself instead, the course page explains
how; the pieces are the GPFSELn function-select table (p. 102), the
alternate-function assignments, and the GPPUDCLKn synopsis (p. 101).

In `kernel/drivers/uart.rs`, implement:

- `uart_init()` — configures the GPIO pins (`gpio_init()`), then the UART:
  disable it, set the baud rate, set 8-bit word length and enable the FIFO,
  finally enable UART + transmit + receive + CTS. Order matters: the device
  must be disabled while configured, and the baud-rate divisors must be set
  *before* the line control register (`LCRH`). The baud divisor is
  `UARTCLK / (16 * baud)`; per the course materials the UART clock is
  48 MHz, so for 115200 baud the divisor is 26.0417 — split into the
  integer (`IBRD` = 26) and fractional (`FBRD` ≈ 3, i.e.
  `round(0.0417 × 64)`) parts per the datasheet.
- `uart_send(c: u8)` — poll the flag register until the transmit FIFO has
  space, then write the byte to `UART_DR`. Polling driver; no interrupts.
- `uart_send_string(s: &core::ffi::CStr)` — send a C string via
  `uart_send`.

Use `unsafe { mmio_read32(addr) }` / `mmio_write32(addr, v)` from
`kernel/utils.rs`, and `delay_cycles` where the datasheet requires waiting.

> **AI use/coding: auxiliary use allowed** — AI may draft pieces, but you
> must write and understand the driver.

### Part C: write `printf`

`kernel/printf.rs` provides the ABI seam:

```rust
pub type VprintfFn = unsafe extern "C" fn(*const c_char, VaList<'_>) -> c_int;

pub unsafe extern "C" fn vprintf(_fmt: *const c_char, _args: VaList<'_>) -> c_int {
    todo!("Quest 2: implement format parsing")  // your work
}

pub unsafe extern "C" fn printf(fmt: *const c_char, args: ...) -> c_int {
    unsafe { vprintf(fmt, args) }
}
```

Implement the format-string state machine inside `vprintf`: normal
characters emit via `uart_send`; `%` enters the specifier state; pull each
argument with `args.next_arg::<T>()` (`VaList` is consumed by mutable
borrow — declare `mut args`). Required specifiers and the Rust type to pull:

| Specifier | `next_arg` type |
|---|---|
| `%d` | `c_int` |
| `%u` | `c_uint` |
| `%ld` | `c_long` |
| `%lu` | `c_ulong` |
| `%x` | `c_uint` (zero-pad to 8 digits) |
| `%lx` | `c_ulong` (zero-pad to 16 digits) |
| `%c` | `c_int` (variadic promotion — not `u8`) |
| `%s` | `*const c_char` |
| `%p` | `*const c_void` (hex; `0x` prefix optional) |
| `%%` | none |

C's default argument promotions apply at the call site: small integer types
(`u8`/`u16`, signed or not, when all values fit) arrive as `c_int` — Rust
callers must cast arguments to the exact expected type. Handle malformed
format strings gracefully. Emit `\r\n` where the C version does (the
terminal expects CRLF). The `%x`/`%lx` zero-padding widths above are this
scaffold's convention, not a claim about the official checker.

`make test-printf` compiles the real `kernel/printf.rs` on the host against
a capture-only mock UART and checks all of the above. It fails now
(`todo!()`); make it pass. Hosted tests don't prove the AArch64 variadic
ABI or hardware behavior — verify on QEMU too.

> **AI use/coding: allowed, including agentic coding** — but you must
> understand the code. Prompt carefully: bare-metal/freestanding, no
> standard library, the exact specifier list, `VaList::next_arg`, output via
> `uart_send`.

### Part D — print π

Set `PI_PRINT` to `true` in `kernel/pi.rs`. `make qemu` should print
`pi ~= 3.1415926535…`.

> **AI use/coding: not allowed** (one token).

**Check:** `make test-qemu-pi` applies the pi serial oracle (all 1,001
digits from `tests/pi_bcd.txt`); `make test-log LOG=<capture> MODE=pi` does
the same for a saved log.

## Quest 3: An OS that runs programs in sequence

**Files:** `kernel/kernel.rs` (`kernel_main`), `kernel/init.rs`,
`kernel/elf.rs`, `kernel/memlayout.rs`, `user/lib/u_common.rs`.

### Background

The three user programs are compiled as **PIE ELF64 executables linked at
virtual address 0** (`user/procs.ld`, `-pie`, no interpreter). The kernel
embeds them in the `.elf_executables` section; `init.rs` exposes them as
`init::SQUARES`, `init::PI`, `init::PRIMECHECK` byte arrays.

The ELF loader in `kernel/elf.rs` is **provided**; its signature is

```rust
pub fn load_elf(image: &[u8], dest: &mut [u8]) -> Result<*const (), ElfError>
```

It copies each `PT_LOAD`, zero-fills the `bss` tail, applies
`R_AARCH64_RELATIVE` relocations (write `base + addend`), and returns
`dest_base + e_entry`. It rejects interpreters, TLS, non-RELATIVE
relocations, overlapping segments, and anything out of bounds — all before
writing a single byte. It does not choose a physical address and does not
execute code; the caller owns the `Result` (including every `Err` case) and
decides when to jump to the entry pointer.

Your `dest` slice must cover the program's virtual range
`[0, max(p_vaddr + p_memsz))`, be aligned to the largest `p_align` in the
file, and be exclusive, valid, initialized `u8` memory while the program
runs — don't mutate or reload it until the program returns.

> **Size matters:** the supplied `user/pi.elf`'s writable `PT_LOAD` spans
> ~28 KiB (`memsz` ≈ 0x6E60 — run `make inspect-users` to see each ELF's
> program headers). An "illustrative" 16 KiB program slot is **not** enough.
> Your `dest` capacity must cover `max(p_vaddr + p_memsz)` over all
> `PT_LOAD`s and its base must satisfy the largest `p_align`; size your
> region from the actual artifacts, then put it on your memory diagram.
>
> Relatedly, the `kernel_end` linker symbol is placed *before* the embedded
> user images: the full kernel footprint runs through
> `elf_executables_end` and includes the user ELFs' debug bytes — well over
> a casual ~1 MiB estimate. Inspect the actual symbols and image size
> (`make inspect-kernel`, `ls -l kernel8.img`) before placing anything after
> the kernel; never assume `kernel_end` is the end of the kernel image.
> Consider this already in Quest 1 when planning where the stack goes — the
> embedded images make the kernel far bigger than its code suggests.

How user programs reach `printf`: `user/lib/u_common.rs` dereferences the
function pointer stored at `F_VPRINTF` (`F_BASE - size_of::<usize>()`, both
in `kernel/memlayout.rs`) and calls it with the user's variadic arguments —
a trampoline to your kernel `vprintf`, matching the `VprintfFn` type alias
in `kernel/printf.rs`. You choose `F_BASE` in Quest 3 and store the address
of `vprintf` at `F_VPRINTF` from `kernel_main` (a volatile `usize` write
through a raw pointer — see `VprintfFn` and the `core::ptr` helpers). The
shim asserts `F_BASE` is a sane aligned nonzero address before dereferencing.
**Keep these guards even after you set `F_BASE`**: they catch a zero or
misaligned `F_BASE`. They do not prove the address is mapped, unoccupied, or
initialized with a valid function pointer; establishing those invariants is
your task. With `F_BASE` still 0, the shim can only reach the panic path —
user programs cannot print until Quest 3 wires it up.

### Tasks

1. **Boot into an actual kernel.** Give your OS a name (**AI use allowed**
   for naming). Write `kernel_main` in `kernel/kernel.rs`, e.g.

   ```rust
   #[unsafe(no_mangle)]
   pub extern "C" fn kernel_main() -> ! { /* your code */ }
   ```

   The `#[unsafe(no_mangle)]` is required or `bl kernel_main` will not
   resolve. `bl` into it from `boot.S` instead of `pi_main`. It should
   `uart_init()`, print a greeting like `"Hello world from <NAME>!\r\n"`,
   and call `pi_main()` to confirm nothing broke.
   (**AI use: auxiliary allowed.**)
2. **Load programs.** Pick a physical program region (your diagram; the C
   handout calls it `PROC_START`), make the exclusive `&mut [u8]` dest
   slice described above, call `load_elf` on each embedded image, handle
   the `Result`, and invoke the returned entry pointer via an
   `extern "C" fn() -> c_int` cast. Initialize the function table in the
   next step before executing a user program for the first time.
   (**AI use: auxiliary allowed** — the loader is provided, the calls are
   yours.)
3. **Library functions.** Set `F_BASE` (a few thousand bytes clear of your
   other regions), update your diagram, and store `vprintf`'s address at
   `F_VPRINTF` in `kernel_main` — see the trampoline description above for
   the operation; upstream explicitly allows AI help for the
   function-pointer incantation.
4. **Batch.** Run `squares`, then `pi`, then `primecheck`, reusing the same
   region (they run to completion one at a time). After the last, halt or
   spin. Note `primecheck`'s loop is finite (`limit = 2 << 27`) — it runs a
   very long time, not literally forever; the 1000th-prime milestone is
   `7919`. `squares` prints `i, i², difference` — the first printed
   difference is 1 (the `0` in the upstream sample output was a typo).
   (**AI use: auxiliary allowed.**)

**Check:** `make test-qemu` (batch oracle: greeting, order, all 100 square
rows, all 1,001 pi digits, first prime milestone) and
`make test-log LOG=<capture>`.

5. **Final:** hand in `docs/memlayout-physical.pdf`. (**AI not allowed.**)

### Bonus quests (extra credit, unchanged)

- **Quest 2-E:** printer control characters in your `printf` (research a
  Tandy DMP-105/DWP-230/DECwriter; **AI allowed**).
- **Quest 3-E:** load ELF programs over the UART console (polling read;
  **AI allowed**). Both require discussing your work with an instructor.

## Handing in & grading

Submission is via the course grading server ("Project 1: Booting") as
described on the course page: connect your GitHub repo, run the server's
submission checks, set your grading commit, and sign up for the 45-minute
Whiteboard discussion (50% of grade; functionality on the server is the
other 50%).

This repository requires the pinned Rust toolchain. Whether the course
grading server can build it is unverified — confirm Rust build support with
course staff (or whether tested commits are evaluated as-is). The local
`make test*` commands are independent scaffold checks, **not** the official
grader and not replacements for the server's checks.

### Extra requirements for CS 1690/2670 students

Your code must work on the **real Raspberry Pi 3B/3B+** (not 4/5). Nothing
in this scaffold touches hardware, disks, or SD cards automatically — the
hardware path stays manual. We have not attached or inspected a physical Pi
or SD card; the procedure below is a vendor/repo-backed setup checklist,
not a hardware-tested promise. Remaining prerequisites are a course-approved
Pi 3 boot/firmware SD card, your completed boot code, and the real UART
test — all noted inline.

**a. SD card.** Start with a course-approved Pi 3 boot/firmware SD card —
you need the *matching* firmware files (`bootcode.bin`, `start.elf`/`fixup`
pair, device trees, …), not just our two outputs. Back up the card's
existing files and verify you are operating on the correct device before
touching anything. The Alpine/Linux seminar demo image is a different thing
and does not prove your kernel works. If your card is unprovisioned, get a
known-good course firmware card/instructions from staff first. This
handout deliberately gives no `dd`/format commands.

**b. Build outputs.** After your code builds, run `make all` (container
`~/projects` or host repo root) — it produces **both** `kernel8.img` and
`armstub.bin`. `kernel8.img` is a raw kernel binary, **not** a partitioned
disk image: do not feed it to Raspberry Pi Imager or `dd` as a whole-disk
image. The firmware expects these as *files on the boot (FAT) partition*.
Build outputs are visible on the host via the bind mount; copy them using
host SD-mount/file tools (the container needs no privileged USB access).
Copy these two project files only after backup/safe identification — exact
copy commands are left to you since mount paths vary.

**c. `config.txt`.** Review the boot partition's `config.txt` for the
boot contract required by this scaffold (advisory settings, not a complete
firmware image or a claim about your card's current contents):

```ini
arm_64bit=1
kernel=kernel8.img
kernel_address=0x80000
armstub=armstub.bin
init_uart_clock=48000000
```

These are the *provided firmware constants* — `kernel_address=0x80000` and
`armstub` naming match the upstream scaffold (the armstub comment's
`KERNEL_ENTRY` is `0x80000`), not student layout choices. `arm_64bit=1`
boots AArch64; `init_uart_clock=48000000` is the documented default UART
clock that the 48 MHz divisor math assumes. Preserve whatever
course-specific settings your card already has; conditional filters,
`os_prefix`, `kernel_old`, initramfs, or Linux-oriented settings require
the course card's own instructions — do not blindly append a block. Do not
add Linux device-tree overlays unless the course requires them. And make
sure the provided `armstub` is actually selected: a default firmware armstub
substitution changes the core/EL boot contract.

**d. Wiring.** Safely eject the card. Keep the Pi unpowered while wiring.
Use a **3.3 V TTL** adapter only — never RS-232 voltage levels — and do not
connect the adapter's power wire when the Pi has its own supply. Common
ground plus crossed TX/RX per the course rules: physical pin 6 = GND,
pin 8 (Pi TX) → adapter RX, pin 10 (Pi RX) → adapter TX. Console:
**115200 8N1**. CTS flow-control requirements are unchanged; the three-wire
wiring above is not a proof of CTS — follow the course UART settings test /
hardware validation rather than guessing extra wiring.

**e. Serial capture.** Run the terminal on the **host** (the container has
no USB passthrough unless you configure one). Identify the actual serial
device yourself — a stable path under `/dev/serial/by-id/…` is preferable —
e.g. `screen /dev/ttyUSB0 115200` (**replace with your real device**;
`screen` may not be installed — use an installed serial terminal, or install
one on your host if needed). Enable the terminal's log capture and
start it *before* powering the Pi. Then evaluate the capture on the host or
copy the log into the project tree and run
`make test-log LOG=<your file> MODE=pi` (or `batch`). Permission-denied or
missing device: check your user's permissions/groups — don't assume `sudo`.
We do not open or reset serial lines automatically.

**f. Reality check.** QEMU cannot verify GPIO wiring, UART voltage levels,
physical CTS signaling, or the SD-card firmware boot sequence. Hardware proof requires the
actual card, wiring, and your driver, plus the course UART test. An
unpowered Pi or an incomplete Quest 1 produces silence; no π output is
expected until Quest 2. Debug Quest 1 in QEMU first.

Checklist:

- [ ] UART config verified — use the grading server's
  **"UART settings test [1690/2670 only]"** button. Real hardware is far
  less forgiving than QEMU: GPIO alt-function selects, baud divisor, and
  CTS flow control must all be right.
- [ ] GPIO: pin 14 = TXD0 (ALT0), pin 15 = RXD0 (ALT0), pin 16 = CTS0 (ALT3),
  pull-up/down clock sequence per datasheet p. 101 — the provided
  `gpio_init` implements this *if you filled in the register offsets*.
- [ ] Serial wiring (3.3 V TTL **only**): Pi pin 6 = GND, pin 8 (Pi TX) →
  adapter RX, pin 10 (Pi RX) → adapter TX; console at 115200 8N1.
- [ ] SD card per steps a–c above: course-approved firmware card, backed up,
  `kernel8.img` + `armstub.bin` copied as files, `config.txt` reviewed.
- [ ] Run on real hardware: your own Pi, the **"Run on hardware
  [1690/2670 only]"** grading-server button, or instructor office hours —
  early, not the last day.
- [ ] Keep the evidence: capture the serial log, and record the artifact
  SHA256 (`sha256sum kernel8.img`), your git commit, and any dirty diff —
  that ties a log to a specific build. `make test-log LOG=<file>` applies
  the output oracle offline; it checks output text only and proves nothing
  about provenance, framing, or registers.

## Rust notes & pitfalls

- Formatting: `cargo fmt --all` (root workspace), and the same inside
  `scaffold-tests/` and `printf-tests/`. Lints: `cargo clippy --workspace`
  (dead-code warnings on the unfinished scaffold are expected).
- Preserve `#[unsafe(no_mangle)]` on symbols reached from assembly or the
  user/kernel ABI (`pi_main`, `pi_bcd`, `spigot_state`, `main`, `printf`,
  `vprintf`, `kernel_main`) — without it the names get mangled and the
  assembler/linker can't find them.
- `static mut` + threads/cores = data races regardless of raw pointers; this
  project runs on one parked-core system so exclusive access is manual.
- The kernel links *fixed* (non-PIE) at `0x80000`; only user programs are
  PIE, and the loader exists because PIE binaries at vaddr 0 need
  `R_AARCH64_RELATIVE` fixups (the old C loader skipped both this and `bss`
  zeroing — the Rust loader does both, validated before any write).
- `make users` before anything that embeds or tests the ELFs;
  `kernel/init.rs` uses `include_bytes!` on `user/*.elf` at compile time.
- Variadic ABI recap: pull `%c` as `c_int` (promoted), `%s` as
  `*const c_char`, `%p` as `*const c_void`; `args` must be `mut`.
- Arithmetic: these development builds **panic on integer overflow**. C
  unsigned arithmetic wraps; C signed overflow is undefined behavior.
  The supplied pi port uses ordinary signed arithmetic, tested at its
  configured digit count. In your own code, use `wrapping_*` only when
  modular arithmetic is intended; `checked_*` and `saturating_*` have
  different semantics. A panic before the UART works looks like a hang.
- Calling into `unsafe` code: calling the provided `pi_main` from Rust
  requires an `unsafe` block and exclusive access to its globals. The
  bounded `load_elf` function itself is safe, but creating a slice over
  chosen physical RAM and converting its returned raw entry address to a
  callable function pointer require you to establish the safety invariants.
- Function pointers: `entry as extern "C" fn() -> c_int` does **not**
  compile. Converting a raw address to a function pointer requires an
  explicit unsafe conversion such as `core::mem::transmute`. The
  `VprintfFn` alias describes the trampoline's function type; it does not
  perform a conversion or establish safety by itself.
- Borrows: end your mutable borrow of the destination before calling the
  loaded program. Its memory must remain reserved and valid throughout
  execution. The program can mutate its own data, but the kernel must not
  concurrently access it or load another program over it.

## Remaining work checklist (for you)

- [ ] `INITIAL_KERNEL_STACK` value + `boot.S` `_start` (Q1), `docs/pi-gdb.png`
- [ ] UART + GPIO register offsets; `uart_init`/`uart_send`/`uart_send_string` (Q2)
- [ ] `vprintf` parser (Q2); `PI_PRINT` flip
- [ ] `kernel_main`, `F_BASE` + `F_VPRINTF` store, `init` batch loader (Q3)
- [ ] `docs/memlayout-physical.pdf`
- [ ] CS 1690/2670 only: UART settings test + real-hardware run + log evidence

## Sources

- Assignment handout (canonical): <https://csci1670.github.io/docs/project1/>
- Testing & debugging: <https://csci1670.github.io/docs/project1/debug/>
- Hardware manuals appendix: <https://csci1670.github.io/docs/project1/manuals/>
- ARM64 reference: <https://csci1670.github.io/docs/project1/arm64/>
- Course hardware info: <https://csci1670.github.io/hardware>
- Serial/seminar notes: <https://csci1670.github.io/docs/seminar01>
- Upstream C ELF loader: <https://github.com/csci1670/cs1670-f26-elfloader>
- Raspberry Pi `config.txt` boot options (vendor docs):
  <https://github.com/raspberrypi/documentation/blob/master/documentation/asciidoc/computers/config_txt/boot.adoc>
  and legacy options incl. `kernel_address`/`init_uart_clock`:
  <https://github.com/raspberrypi/documentation/blob/master/documentation/asciidoc/computers/legacy_config_txt/boot.adoc>

*Handout text adapted from the course materials under CC BY 4.0.*
