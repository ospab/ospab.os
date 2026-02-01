fn main() {
    // Inform cargo to rerun if linker script changes
    println!("cargo:rerun-if-changed=linker.ld");
}
