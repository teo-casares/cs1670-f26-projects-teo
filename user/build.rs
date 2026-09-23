fn main() {
    // -nmagic: avoid page alignment of segments in ELF executables (ARM64
    // requires 64kB alignment, which wastes a lot of space)
    // -T user/procs.ld: use custom linker script to define memory layout.
    // rustc runs from the workspace root, so a workspace-relative path keeps
    // cached build artifacts free of host-specific absolute paths.
    println!("cargo:rustc-link-arg=-Tuser/procs.ld");
    println!("cargo:rustc-link-arg=-pie");
    println!("cargo:rustc-link-arg=--no-dynamic-linker");
    println!("cargo:rustc-link-arg=-nmagic");
    println!("cargo:rustc-link-arg=-Bsymbolic");
    println!("cargo:rerun-if-changed=procs.ld");
}
