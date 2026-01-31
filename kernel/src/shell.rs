//! Shell implementation for ospabOS

use crate::drivers::vga_buffer;
use spin::{Mutex, Once};

static SHELL_ONCE: Once<Mutex<Shell>> = Once::new();

pub fn shell() -> &'static Mutex<Shell> {
    SHELL_ONCE.call_once(|| Mutex::new(Shell::new()))
}

pub struct Shell {
    buf: [u8; 128],
    len: usize,
}

impl Shell {
    pub fn new() -> Self {
        Shell { buf: [0u8; 128], len: 0 }
    }

    pub fn on_keypress(&mut self, c: char) {
        match c {
            '\n' => {
                self.execute_command();
                self.len = 0;
            }
            '\x08' => {
                if self.len > 0 {
                    self.len -= 1;
                    vga_buffer::backspace();
                }
            }
            _ => {
                if self.len < self.buf.len() {
                    self.buf[self.len] = c as u8;
                    self.len += 1;
                    vga_buffer::put_char(c);
                }
            }
        }
    }

    pub fn execute_command(&self) {
        let cmd = core::str::from_utf8(&self.buf[..self.len]).unwrap_or("");
        match cmd {
            "help" => vga_buffer::print("Available commands: help, clear, status, ping\n"),
            "clear" => vga_buffer::init(),
            "status" => vga_buffer::print("ospabOS Kernel is running smoothly.\n"),
            "ping" => vga_buffer::print("Pong!\n"),
            _ => vga_buffer::print("Unknown command. Type 'help' for a list of commands.\n"),
        }
        // print prompt after command
        vga_buffer::print("> ");
    }
}