use std::env;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let linker = Path::new(&manifest_dir).join("linker.ld");
    println!("cargo:rustc-link-arg=-T{}", linker.display());
    println!("cargo:rustc-link-arg=--no-dynamic-linker");
    println!("cargo:rustc-link-arg=-static");
    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-changed=boot.S");
    println!("cargo:rerun-if-changed=mm.S");
    println!("cargo:rerun-if-changed=../user/squares.elf");
    println!("cargo:rerun-if-changed=../user/pi.elf");
    println!("cargo:rerun-if-changed=../user/primecheck.elf");
}
