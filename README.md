# CS 1670/2670 Projects

This repository contains the project code for Brown's CS 1670/2670 (CSCI 1670/2670) class, Operating Systems (Fall 2026 edition).

For more information, visit: http://cs1670.systems/

## Rust scaffold (Project 1)

Project 1 in this repository has been translated to Rust. The original C
stencil files (`.c`, `.h`, generator scripts, `user/*.c`, `kernel/*.c`) are
kept in the tree as **inactive upstream references** — they are not built.
The assignment write-up, translated to Rust, is in
[`docs/project1-rust.md`](docs/project1-rust.md).

### Layout

- `kernel/` — kernel crate (`kernel.rs`, `boot.S`, `mm.S`, `linker.ld`,
  `memlayout.rs`, `drivers/`, `utils.rs`, `printf.rs`, `pi.rs`, `elf.rs`,
  `init.rs`)
- `user/` — user-program crate (`squares.rs`, `pi.rs`, `primecheck.rs`,
  `lib/u_common.rs`, `procs.ld`) and the produced `user/*.elf` images
- `bootloader/` — armstub crate (`armstub.rs`, `armstub.S`, `linker.ld`)
- `scaffold-tests/` — host-side tests for the provided ELF loader and the
  supplied pi port (not part of the AArch64 workspace)
- `printf-tests/` — hosted test for the student-implemented printf
  (expected to fail until Quest 2)
- `scripts/check_output.py` — output oracle used by `test-qemu`/`test-log`
- `tests/` — frozen oracle fixtures (`pi_bcd.txt`) and oracle unit tests

### Toolchain

The build uses a pinned host toolchain (see `rust-toolchain.toml`):
`beta-2026-09-20` with the `aarch64-unknown-none-softfloat` target and the
`rustfmt`, `clippy`, `llvm-tools` components. Install it once with:

```sh
rustup toolchain install beta-2026-09-20 --profile minimal \
  -c rustfmt -c clippy -c llvm-tools -t aarch64-unknown-none-softfloat
```

QEMU (`qemu-system-aarch64`), `make`, `python3`, and GDB are needed for
`make qemu`/`make qemu-gdb` and the test targets (plain `gdb` if it supports
AArch64, `gdb-multiarch` otherwise; connect with `gdb -nx -x .gdbinit`).
The course Docker container has the cross GCC/GDB toolchain but **no Rust**;
run the Rust build on the host (or install rustup inside your container).

Build order matters: run `make users` first — it produces `user/*.elf`,
which the kernel embeds and the host tests read. Plain `cargo build` does
not refresh those artifacts.

Quick start (path shown for this machine; adjust for your own host):

```sh
cd /home/teo/cs1670/container-home/projects
make check
make test
```

### Commands

Run `make help` for the full list. In short:

- `make users`, `make check`, `make test`, `make armstub.bin` work on the
  untouched scaffold.
- `make kernel8.img` intentionally fails until you implement `_start` in
  `kernel/boot.S` (Quest 1) — the linker prints
  `Quest 1: implement _start in kernel/boot.S`.
- `make test-printf` fails until you implement the formatter (Quest 2).
- `make qemu`, `make test-qemu`, `make test-log LOG=...` are for after the
  kernel boots.
