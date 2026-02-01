//! Keyboard driver for ospabOS

#![allow(static_mut_refs)]

use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use x86_64::instructions::port::Port;
use crate::drivers::framebuffer;

// Local serial print for debugging
fn serial_print(msg: &[u8]) {
    let mut port: Port<u8> = Port::new(0x3F8);
    for &b in msg {
        unsafe { port.write(b); }
    }
}

#[allow(dead_code)]
fn serial_print_hex(val: u8) {
    const HEX: &[u8] = b"0123456789ABCDEF";
    serial_print(&[HEX[(val >> 4) as usize], HEX[(val & 0xF) as usize]]);
}

const CMD_BUFFER_SIZE: usize = 256;
const SCANCODE_BUFFER_SIZE: usize = 128;

pub struct KeyboardState {
    keyboard: Option<Keyboard<layouts::Us104Key, ScancodeSet1>>,
    cmd_buf: [u8; CMD_BUFFER_SIZE],
    cmd_len: usize,
    // Ring buffer for scancodes from ISR
    scancode_buf: [u8; SCANCODE_BUFFER_SIZE],
    scancode_read: usize,
    scancode_write: usize,
}

static mut STATE: KeyboardState = KeyboardState {
    keyboard: None,
    cmd_buf: [0u8; CMD_BUFFER_SIZE],
    cmd_len: 0,
    scancode_buf: [0u8; SCANCODE_BUFFER_SIZE],
    scancode_read: 0,
    scancode_write: 0,
};

pub fn init() {
    unsafe {
        STATE.keyboard = Some(Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore));
    }
}

/// Called from ISR - just queue the scancode
pub fn queue_scancode(scancode: u8) {
    unsafe {
        let next_write = (STATE.scancode_write + 1) % SCANCODE_BUFFER_SIZE;
        // Only add if buffer not full (leave one slot empty for full/empty detection)
        if next_write != STATE.scancode_read {
            STATE.scancode_buf[STATE.scancode_write] = scancode;
            STATE.scancode_write = next_write;
        }
        // If buffer full, silently drop scancode (better than crashing)
    }
}

/// Called from main loop - process queued scancodes
pub fn process_scancodes() {
    unsafe {
        // Process up to 16 scancodes per call to avoid blocking too long
        let mut count = 0;
        while STATE.scancode_read != STATE.scancode_write && count < 16 {
            let scancode = STATE.scancode_buf[STATE.scancode_read];
            STATE.scancode_read = (STATE.scancode_read + 1) % SCANCODE_BUFFER_SIZE;
            handle_scancode(scancode);
            count += 1;
        }
    }
}

pub fn handle_scancode(scancode: u8) {
    unsafe {
        let kb = match STATE.keyboard.as_mut() {
            Some(k) => k,
            None => return, // Not initialized yet
        };
        
        let key_event = match kb.add_byte(scancode) {
            Ok(Some(ev)) => ev,
            _ => return,
        };
        
        let key = match kb.process_keyevent(key_event) {
            Some(k) => k,
            None => return,
        };
        
        match key {
            DecodedKey::Unicode(character) => {
                handle_char(character);
            }
            DecodedKey::RawKey(_k) => {}
        }
    }
}

fn handle_char(c: char) {
    unsafe {
        match c {
            '\n' | '\r' => {
                framebuffer::print_char('\n');
                execute_command();
                STATE.cmd_len = 0;
                framebuffer::print("[ospab]~> ");
            }
            '\x08' => {
                // Backspace
                if STATE.cmd_len > 0 {
                    STATE.cmd_len -= 1;
                    framebuffer::print_char('\x08');
                }
            }
            c if c.is_ascii() && !c.is_control() => {
                if STATE.cmd_len < CMD_BUFFER_SIZE - 1 {
                    STATE.cmd_buf[STATE.cmd_len] = c as u8;
                    STATE.cmd_len += 1;
                    framebuffer::print_char(c);
                }
            }
            _ => {}
        }
    }
}

fn execute_command() {
    unsafe {
        let cmd_bytes = &STATE.cmd_buf[..STATE.cmd_len];
        let cmd = match core::str::from_utf8(cmd_bytes) {
            Ok(s) => s.trim(),
            Err(_) => {
                framebuffer::print("Error: invalid UTF-8\n");
                return;
            }
        };
        
        match cmd {
            "" => {}
            "help" => {
                framebuffer::print("Available commands:\n");
                framebuffer::print("  help    - Show this help\n");
                framebuffer::print("  clear   - Clear screen\n");
                framebuffer::print("  status  - Show system status\n");
                framebuffer::print("  about   - About ospabOS\n");
                framebuffer::print("  reboot  - Reboot system\n");
            }
            "clear" => {
                // Skip clear - it's slow
                framebuffer::print("(clear disabled - slow)\n");
            }
            "status" => {
                framebuffer::print("System Status: Running\n");
                framebuffer::print("Kernel: ospabOS v0.1.0\n");
                framebuffer::print("Arch: x86_64\n");
            }
            "about" => {
                framebuffer::print("\n");
                framebuffer::print("  ospabOS - A hobby operating system\n");
                framebuffer::print("  Written in Rust, booted via Limine\n");
                framebuffer::print("  Version 0.1.0\n");
                framebuffer::print("\n");
            }
            "reboot" => {
                framebuffer::print("Rebooting...\n");
                let mut port = Port::<u8>::new(0x64);
                port.write(0xFE);
            }
            _ => {
                framebuffer::print("Error: unknown command '");
                framebuffer::print(cmd);
                framebuffer::print("'\n");
                framebuffer::print("Type 'help' for available commands.\n");
            }
        }
    }
}