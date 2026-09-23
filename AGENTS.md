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
- `make inspect-users` / `make inspect-kernel` — `llvm-readobj`/`llvm-nm`
  dumps of the user ELFs (works now) and the linked kernel (needs Quest 1).
  The pinned toolchain ships `llvm-readobj`, not `llvm-readelf`.
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

## Runtime notes (learned)

- Host and `cs1670-container` are separate environments sharing the repo via
  bind mount: host path `/home/teo/cs1670/container-home/projects` ==
  container `~/projects` (user `cs1670-user`, uid/gid 1000). Do not copy
  toolchains between them.
- Rust lives in the container user's home (`~/.cargo`, `~/.rustup`;
  `~/.bashrc` sources `~/.cargo/env` for interactive shells). For
  non-interactive checks:
  `docker exec -u cs1670-user -w /home/cs1670-user/projects cs1670-container sh -lc '. "$HOME/.cargo/env"; make check && make test'`
- `*/build.rs` pass workspace-root-relative `-T` linker script paths on
  purpose: rustc runs from the workspace root, and absolute
  `CARGO_MANIFEST_DIR` paths baked into the shared `target/` cache break
  whichever side (host vs container) did not produce them. Keep it relative.
- Container has `qemu-system-aarch64`, `gdb-multiarch`, `make`, `python3`,
  `gcc` — but no `aarch64-none-elf-gdb`; use `gdb-multiarch -nx -x .gdbinit`.
- Clean-room verification recipe: copy the source (excluding `target/`,
  `user/*.elf`, caches) to a scratch dir under the bind-mounted home, then
  in the container `make clean && make check && make test && make
  armstub.bin && make inspect-users` — proves the tree builds from nothing.
- Hardware boundary: never flash/format SD cards, open serial devices, or
  run hardware operations for the user — the SD/`config.txt`/UART steps in
  the handout are manual student/course tasks.
- Reminder: do not complete the student exercises (boot, UART, printf,
  kernel_main/init) while "fixing" anything — see task boundaries above.
