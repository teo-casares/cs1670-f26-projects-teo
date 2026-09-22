# Agent notes — Project 1 Rust scaffold

This repo is a Rust-native translation of the CS 1670/2670 Project 1
stencil. The assignment write-up is `docs/project1-rust.md`; the canonical
course page is https://csci1670.github.io/docs/project1/ (CC BY 4.0).

## Build & test

- Pinned toolchain: `beta-2026-09-20` (`rust-toolchain.toml`), target
  `aarch64-unknown-none-softfloat`, components rustfmt/clippy/llvm-tools.
- `make users` — build + copy `user/*.elf` (required before kernel builds,
  since `kernel/init.rs` embeds them).
- `make check` — `cargo check --workspace`. Passes on the scaffold.
- `make test` — host tests in `scaffold-tests/` (ELF loader incl. real
  `user/*.elf`, pi port vs. `tests/pi_bcd.txt`) + Python oracle unit tests.
- `make test-printf` — `printf-tests/` hosted formatter test; EXPECTED TO
  FAIL until the student implements `vprintf` (Quest 2). Do not "fix" it by
  implementing the parser.
- `make kernel8.img` — EXPECTED TO FAIL with
  `Quest 1: implement _start in kernel/boot.S`. That diagnostic is the
  intended state of the scaffold; do not add a `_start` to silence it.
- `make armstub.bin` — must stay byte-identical to the upstream GCC build
  (`llvm-objcopy -O binary bootloader/armstub.elf` is the reference).
- `make test-qemu*` / `make test-log` — post-boot checks via
  `scripts/check_output.py`, a local scaffold oracle authored for this Rust
  port (not official course tests — do not edit pass/fail logic).

## Task boundaries (do not implement student exercises)

- No `_start`/boot instructions in `kernel/boot.S` (Quest 1).
- No real UART/GPIO register offsets or driver bodies (Quest 2);
  `kernel/drivers/{uart,gpio}.rs` keep the upstream `+ 0x0` placeholders.
- No printf format parser in `kernel/printf.rs` (Quest 2).
- No `kernel_main`/`init` program-execution logic (Quest 3); `init.rs` only
  embeds the images.
- Do not choose memory addresses (`INITIAL_KERNEL_STACK`, `F_BASE`, program
  load regions) — those are student decisions; placeholders stay.
- Original `.c`/`.h` files and `gen_klib_header.py` are inactive upstream
  references — do not delete or wire them into the build.

## Layout

- Workspace members: `kernel`, `user` (package `user-programs`),
  `bootloader` (package `armstub`). `default-members = ["kernel"]`.
- `scaffold-tests/` and `printf-tests/` are host-target crates *excluded*
  from the workspace (own `[workspace]` table). They must be built with an
  explicit host target so they also work on macOS/ARM hosts:
  `cargo test --manifest-path scaffold-tests/Cargo.toml --target $(rustc -vV | sed -n 's/^host: //p')`
  and the same for `printf-tests` (this is what `make test`/`make
  test-printf` do).
- `kernel/memlayout.rs` is shared by the user lib via `#[path]` — one source
  for `F_BASE`/`F_VPRINTF`.
- `user/procs.ld` links PIE at vaddr 0 with `PT_LOAD`/`PT_DYNAMIC`; the
  loader in `kernel/elf.rs` copies `PT_LOAD`, zeros `bss`, and applies
  `R_AARCH64_RELATIVE`.
