fn main() {
    // Inform cargo to rerun if linker script changes
    println!("cargo:rerun-if-changed=linker.ld");

    // Build doomgeneric C sources (vendor)
    let mut build = cc::Build::new();
    build.include("src/doomgeneric");
    build.include("src/vendor/doomgeneric");
    // Add our glue
    build.file("src/doomgeneric/doomgeneric_syscalls.c");
    // Add all .c files from vendor doomgeneric
    for entry in std::fs::read_dir("src/vendor/doomgeneric").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "c" {
                build.file(path);
            }
        }
    }
    build.compile("doomgeneric");
}
