fn main() {
    println!("cargo:rerun-if-changed=src/boot/limine_header.S");
    cc::Build::new()
        .file("src/boot/limine_header.S")
        .compile("limine_header");
}
