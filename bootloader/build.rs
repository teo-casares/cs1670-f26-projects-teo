fn main() {
    println!("cargo:rustc-link-arg=-Tbootloader/linker.ld");
    println!("cargo:rustc-link-arg=--no-dynamic-linker");
    println!("cargo:rustc-link-arg=-static");
    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-changed=armstub.S");
}
