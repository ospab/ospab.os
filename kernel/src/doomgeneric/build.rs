// build.rs for doomgeneric FFI integration
fn main() {
    cc::Build::new()
        .file("src/doomgeneric/doomgeneric.c")
        .file("src/doomgeneric/doomgeneric_syscalls.c")
        .include("src/doomgeneric")
        .compile("doomgeneric");
}
