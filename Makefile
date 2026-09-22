.PHONY: all clean format help users check test test-printf test-log test-qemu test-qemu-pi test-suite armstub.bin

.DEFAULT_GOAL := all

HOST_OS = $(shell uname -s)
HOST_ARCH = $(shell uname -m)
HOST_TRIPLE = $(shell rustc -vV | sed -n 's/^host: //p')

ifndef IN_CONTAINER
ifneq ("$(wildcard /etc/image-version)","")
    IN_CONTAINER = 1
else
    IN_CONTAINER = 0
endif
endif

PYTHON ?= python3
QEMU ?= qemu-system-aarch64
TEST_TIMEOUT ?= 120
MODE ?= batch
TARGET = aarch64-unknown-none-softfloat
TARGET_DIR = target/$(TARGET)/debug

# LLVM binutils shipped with the pinned Rust toolchain (llvm-tools component).
LLVM_BIN = $(shell rustc --print sysroot)/lib/rustlib/$(HOST_TRIPLE)/bin
OBJCOPY = $(LLVM_BIN)/llvm-objcopy

K = kernel
U = user

all: kernel8.img armstub.bin

help:
	@printf '%s\n' \
	  "Project 1 (Rust scaffold) targets:" \
	  "  make users        build the user programs (user/*.elf)" \
	  "  make check        cargo check --workspace (no kernel link)" \
	  "  make test         host-side scaffold tests (ELF loader, pi port, oracle)" \
	  "  make test-printf  hosted printf test (fails until Quest 2)" \
	  "  make test-qemu    QEMU batch-output check (needs Quest 3 done)" \
	  "  make test-qemu-pi QEMU pi serial-output check (needs Quest 2 done)" \
	  "  make test-log LOG=<file> [MODE=batch|pi]  check a captured log" \
	  "  make test-suite   test + test-printf + test-qemu" \
	  "  make kernel8.img  link the kernel (fails until _start exists)" \
	  "  make armstub.bin  build the bootloader blob" \
	  "  make qemu         run kernel8.img in QEMU (see qemu.mk)" \
	  "  make format       cargo fmt --all" \
	  "  make clean        remove generated artifacts"

###################
# user programs
###################

users:
	cargo build -p user-programs
	cp $(TARGET_DIR)/squares   $(U)/squares.elf
	cp $(TARGET_DIR)/pi        $(U)/pi.elf
	cp $(TARGET_DIR)/primecheck $(U)/primecheck.elf

###################
# checks and tests
###################

check: users
	cargo check --workspace

test: users
	cargo test --manifest-path scaffold-tests/Cargo.toml --target $(HOST_TRIPLE)
	$(PYTHON) -m unittest discover -s tests -p 'test_*.py'

# Hosted printf formatter test. Runs the REAL kernel/printf.rs against a
# capture-only mock UART. Expected to FAIL until Quest 2 is implemented.
test-printf: users
	cargo test --manifest-path printf-tests/Cargo.toml --target $(HOST_TRIPLE) -- --test-threads=1

# Offline check of a captured serial/QEMU log. Reads only the provided file;
# passing does not prove the log came from real hardware.
test-log:
	@test -n "$(LOG)" || { echo "usage: make test-log LOG=<serial/qemu log file> [MODE=batch|pi]"; exit 2; }
	$(PYTHON) scripts/check_output.py --log "$(LOG)" --mode "$(MODE)"

# Boot the kernel in QEMU and apply the batch/pi output oracle to the captured
# serial stream. Expected to fail until _start (and the rest) are implemented.
test-qemu: kernel8.img
	$(PYTHON) scripts/check_output.py --qemu --mode batch --timeout $(TEST_TIMEOUT)

test-qemu-pi: kernel8.img
	$(PYTHON) scripts/check_output.py --qemu --mode pi --timeout $(TEST_TIMEOUT)

test-suite: test test-printf test-qemu

###################
# kernel image
###################

kernel8.img: users
	cargo build -p kernel
	$(OBJCOPY) $(TARGET_DIR)/kernel -O binary $@

###################
# bootloader
###################

# Bootloader for the Raspberry Pi 3 that sets up tick counter and other hardware configuration.
# The firmware loads this at address 0x0, and it eventually hands over control to the code in
# boot.S.
armstub.bin:
	cargo build -p armstub
	$(OBJCOPY) $(TARGET_DIR)/armstub -O binary $@

###################
# housekeeping
###################

format:
	cargo fmt --all
	cd scaffold-tests && cargo fmt --all
	cd printf-tests && cargo fmt --all

clean:
	rm -f kernel8.img armstub.bin
	rm -f $(U)/squares.elf $(U)/pi.elf $(U)/primecheck.elf
	cargo clean
	cd scaffold-tests && cargo clean
	cd printf-tests && cargo clean

###################
# QEMU
###################

include qemu.mk
