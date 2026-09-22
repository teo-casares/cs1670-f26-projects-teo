use std::env;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let linker = Path::new(&manifest_dir).join("procs.ld");
    // -nmagic: avoid page alignment of segments in ELF executables (ARM64
    // requires 64kB alignment, which wastes a lot of space)
    // -T procs.ld: use custom linker script to define memory layout
    println!("cargo:rustc-link-arg=-T{}", linker.display());
    println!("cargo:rustc-link-arg=-pie");
    println!("cargo:rustc-link-arg=--no-dynamic-linker");
    println!("cargo:rustc-link-arg=-nmagic");
    println!("cargo:rustc-link-arg=-Bsymbolic");
    println!("cargo:rerun-if-changed=procs.ld");
}
