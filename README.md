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

- `docs/` — Rust handout and blank editable `memlayout-physical.svg`;
  draw your own diagram and later save a PDF copy as `memlayout-physical.pdf`
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

The base course Docker image does **not** include Rust, and the host and the
container are separate installs — rustup on the host does not exist inside
`cs1670-container` and vice versa (never copy `~/.cargo`/`~/.rustup` between
them). In this setup rustup is installed into the container user's home
(`~/.cargo`, `~/.rustup`), which is bind-mounted from
`/home/teo/cs1670/container-home`, so it persists across container restarts.
On a *fresh* container/home there is no Rust: install it per
<https://rustup.rs> and then run the pinned toolchain command above.

Build order matters: run `make users` first — it produces `user/*.elf`,
which the kernel embeds and the host tests read. Plain `cargo build` does
not refresh those artifacts.

Quick start on the host (path shown for this machine; adjust for yours):

```sh
cd /home/teo/cs1670/container-home/projects
make check
make test
```

Container workflow — it is **one shared bind-mounted checkout** (the same
files appear at both paths) with **separate toolchains** on each side.
`./run-container` is a script in the parent directory, not a PATH command —
run it from `/home/teo/cs1670`.

Host terminal 1 (launches/attaches the container):

```sh
cd /home/teo/cs1670
./run-container
```

Inside the container shell (an already-open shell needs one manual
`source`; new login shells pick it up from `~/.bashrc` automatically):

```sh
source "$HOME/.cargo/env"   # only if cargo isn't found yet
cd ~/projects
make check
make test
```

After Quest 1, for `make qemu-gdb` you need a second container shell —
open host terminal 2 and repeat `./run-container` (it attaches to the same
running container), then:

```sh
source "$HOME/.cargo/env"   # same one-time fix if needed
cd ~/projects
gdb-multiarch -nx -x .gdbinit
```

The container's plain `gdb` is host-architecture only; use `gdb-multiarch`.
QEMU listens on GDB port 1234 by default — run `make qemu-gdb` and the
debugger in the *same* environment (both inside the container, or both on
the host).

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
