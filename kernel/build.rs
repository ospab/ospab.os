fn main() {
    // Inform cargo to rerun if linker script changes
    println!("cargo:rerun-if-changed=linker.ld");

    // Build doomgeneric C sources (vendor)
    let mut build = cc::Build::new();
    build.include("src/doomgeneric");
    build.include("src/vendor/doomgeneric");
    // Add our glue
    build.file("src/doomgeneric/doomgeneric_syscalls.c");
    // Add ospab port glue
    build.file("src/doomgeneric/doomgeneric_ospab.c");
    // Add libc compat helpers (stdio/malloc wrappers)
    build.file("src/doomgeneric/libc_compat.c");
    // Add all .c files from vendor doomgeneric
    // Recursively add all .c files under vendor/doomgeneric
    fn add_c_files(dir: &std::path::Path, build: &mut cc::Build) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                add_c_files(&path, build);
            } else if let Some(ext) = path.extension() {
                if ext == "c" {
                    let name = path.file_name().unwrap().to_string_lossy().to_lowercase();
                    // Skip platform-specific/front-end files that require external libraries
                    if name.contains("allegro") || name.contains("sdl") || name.contains("win") || name.contains("xlib") || name.contains("soso") || name.contains("sosox") || name.contains("emscripten") {
                        continue;
                    }
                    build.file(path);
                }
            }
        }
    }
    add_c_files(std::path::Path::new("src/vendor/doomgeneric"), &mut build);

    // Suppress warnings from vendor code
    build.flag("-w"); // Disable all warnings from C code
    build.flag("-Wno-unused-parameter");
    build.flag("-Wno-sign-compare");
    build.flag("-Wno-unused-variable");
    build.flag("-Wno-implicit-function-declaration");
    build.flag("-Wno-unused-function");
    build.flag("-Wno-unused-but-set-variable");
    build.flag("-Wno-missing-field-initializers");

    build.compile("doomgeneric");
}
