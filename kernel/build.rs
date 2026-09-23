fn main() {
    println!("cargo:rustc-link-arg=-Tkernel/linker.ld");
    println!("cargo:rustc-link-arg=--no-dynamic-linker");
    println!("cargo:rustc-link-arg=-static");
    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-changed=boot.S");
    println!("cargo:rerun-if-changed=mm.S");
    println!("cargo:rerun-if-changed=../user/squares.elf");
    println!("cargo:rerun-if-changed=../user/pi.elf");
    println!("cargo:rerun-if-changed=../user/primecheck.elf");
}
