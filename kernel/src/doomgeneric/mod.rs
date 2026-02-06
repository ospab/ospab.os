//! DOOMGENERIC integration for ospabOS
//! This module will wrap doomgeneric C code and expose Rust interface

pub mod ffi;

#[link(name = "doomgeneric")]
extern "C" {
    fn doomgeneric_main();
}

pub fn run() {
    unsafe {
        doomgeneric_main();
    }
}
