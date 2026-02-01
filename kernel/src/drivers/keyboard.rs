//! Keyboard driver for ospabOS

use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::{Mutex, Once};
use x86_64::instructions::port::Port;
use crate::drivers::framebuffer;

static KEYBOARD_ONCE: Once<Mutex<KeyboardDriver>> = Once::new();

pub fn keyboard() -> &'static Mutex<KeyboardDriver> {
    KEYBOARD_ONCE.call_once(|| Mutex::new(KeyboardDriver::new()))
}

const CMD_BUFFER_SIZE: usize = 256;

pub struct KeyboardDriver {
    keyboard: Keyboard<layouts::Us104Key, ScancodeSet1>,
    scancode_port: Port<u8>,
    // Command buffer for shell
    cmd_buf: [u8; CMD_BUFFER_SIZE],
    cmd_len: usize,
}

impl KeyboardDriver {
    pub fn new() -> Self {
        KeyboardDriver {
            keyboard: Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore),
            scancode_port: Port::new(0x60),
            cmd_buf: [0u8; CMD_BUFFER_SIZE],
            cmd_len: 0,
        }
    }

    pub fn handle_interrupt(&mut self) {
        let scancode: u8 = unsafe { self.scancode_port.read() };
        if let Ok(Some(key_event)) = self.keyboard.add_byte(scancode) {
            if let Some(key) = self.keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => self.handle_char(character),
                    DecodedKey::RawKey(_k) => {}
                }
            }
        }
    }

    fn handle_char(&mut self, c: char) {
        match c {
            '\n' | '\r' => {
                framebuffer::print_char('\n');
                self.execute_command();
                self.cmd_len = 0;
                framebuffer::print("> ");
            }
            '\x08' => {
                // Backspace
                if self.cmd_len > 0 {
                    self.cmd_len -= 1;
                    framebuffer::print_char('\x08');
                }
            }
            c if c.is_ascii() && !c.is_control() => {
                if self.cmd_len < CMD_BUFFER_SIZE - 1 {
                    self.cmd_buf[self.cmd_len] = c as u8;
                    self.cmd_len += 1;
                    framebuffer::print_char(c);
                }
            }
            _ => {}
        }
    }

    fn execute_command(&mut self) {
        let cmd = core::str::from_utf8(&self.cmd_buf[..self.cmd_len]).unwrap_or("");
        let cmd = cmd.trim();
        
        match cmd {
            "" => {}
            "help" => {
                framebuffer::print("Available commands:\n");
                framebuffer::print("  help    - Show this help\n");
                framebuffer::print("  clear   - Clear screen\n");
                framebuffer::print("  status  - Show system status\n");
                framebuffer::print("  meminfo - Show memory info\n");
                framebuffer::print("  about   - About ospabOS\n");
                framebuffer::print("  reboot  - Reboot system\n");
            }
            "clear" => {
                framebuffer::clear();
            }
            "status" => {
                framebuffer::print("System Status: Running\n");
                framebuffer::print("Kernel: ospabOS v0.1.0\n");
                framebuffer::print("Arch: x86_64\n");
            }
            "meminfo" => {
                if let Some(memmap) = crate::boot::memory_map() {
                    framebuffer::print("Memory Map:\n");
                    let mut _usable_total: u64 = 0;
                    for entry in memmap.iter().take(10) {
                        let typ_str = match entry.typ {
                            0 => "Usable",
                            1 => "Reserved",
                            2 => "ACPI Reclaimable",
                            3 => "ACPI NVS",
                            4 => "Bad Memory",
                            5 => "Bootloader",
                            6 => "Kernel",
                            7 => "Framebuffer",
                            _ => "Unknown",
                        };
                        if entry.typ == 0 {
                            _usable_total += entry.length;
                        }
                        // Simple output without formatting
                        framebuffer::print("  ");
                        framebuffer::print(typ_str);
                        framebuffer::print("\n");
                    }
                    framebuffer::print("(More entries may exist)\n");
                } else {
                    framebuffer::print("Memory map not available\n");
                }
            }
            "about" => {
                framebuffer::print("\n");
                framebuffer::print("  ___  ____  ___   __   ___   ___  ____\n");
                framebuffer::print(" / _ \\/ ___||  _ \\ / _\\ | _ ) / _ \\/ ___|\n");
                framebuffer::print("| | | \\___ \\|  __/| |_| | _ \\| | | \\___ \\\n");
                framebuffer::print("| |_| |___) |_|   |  _| | |_) | |_| |___) |\n");
                framebuffer::print(" \\___/|____/      |_|   |___/ \\___/|____/\n");
                framebuffer::print("\n");
                framebuffer::print("ospabOS - A hobby operating system\n");
                framebuffer::print("Written in Rust, booted via Limine\n");
                framebuffer::print("Version 0.1.0\n");
                framebuffer::print("\n");
            }
            "reboot" => {
                framebuffer::print("Rebooting...\n");
                // Triple fault to reboot
                unsafe {
                    // Reset via 8042 keyboard controller
                    let mut port = Port::<u8>::new(0x64);
                    port.write(0xFE);
                }
            }
            _ => {
                framebuffer::print("Unknown command: ");
                framebuffer::print(cmd);
                framebuffer::print("\nType 'help' for available commands.\n");
            }
        }
    }
}