//! Keyboard driver for ospabOS

use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::{Mutex, Once};
use x86_64::instructions::port::Port;

static KEYBOARD_ONCE: Once<Mutex<KeyboardDriver>> = Once::new();

pub fn keyboard() -> &'static Mutex<KeyboardDriver> {
    KEYBOARD_ONCE.call_once(|| Mutex::new(KeyboardDriver::new()))
}

pub struct KeyboardDriver {
    keyboard: Keyboard<layouts::Us104Key, ScancodeSet1>,
    scancode_port: Port<u8>,
    // simple input buffer for decoded characters
    input_buf: [u8; 256],
    head: usize,
    tail: usize,
}

impl KeyboardDriver {
    pub fn new() -> Self {
        KeyboardDriver {
            keyboard: Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore),
            scancode_port: Port::new(0x60),
            input_buf: [0u8; 256],
            head: 0,
            tail: 0,
        }
    }

    pub fn handle_interrupt(&mut self) {
        let scancode: u8 = unsafe { self.scancode_port.read() };
        if let Ok(Some(key_event)) = self.keyboard.add_byte(scancode) {
            if let Some(key) = self.keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => self.push_char(character),
                    DecodedKey::RawKey(_k) => {}
                }
            }
        }
    }

    fn push_char(&mut self, c: char) {
        let next = (self.head + 1) & 255;
        if next != self.tail {
            self.input_buf[self.head] = c as u8;
            self.head = next;
        }
    }

    pub fn get_char(&mut self) -> Option<char> {
        if self.tail == self.head {
            return None;
        }
        let b = self.input_buf[self.tail];
        self.tail = (self.tail + 1) & 255;
        Some(b as char)
    }
}