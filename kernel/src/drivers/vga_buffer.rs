use core::fmt;
use spin::Mutex;

const VGA_BUFFER: *mut u16 = 0xB8000 as *mut u16;
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;

#[repr(u8)]
pub enum Color {
    Black = 0x0,
    Blue = 0x1,
    Green = 0x2,
    Cyan = 0x3,
    Red = 0x4,
    Magenta = 0x5,
    Brown = 0x6,
    LightGray = 0x7,
    DarkGray = 0x8,
    LightBlue = 0x9,
    LightGreen = 0xA,
    LightCyan = 0xB,
    LightRed = 0xC,
    Pink = 0xD,
    Yellow = 0xE,
    White = 0xF,
}

const fn color_code(fg: Color, bg: Color) -> u8 {
    ((bg as u8) << 4) | (fg as u8)
}

pub struct Writer {
    column_position: usize,
    color_code: u8,
    buffer: *mut u16,
}

unsafe impl Send for Writer {}

impl Writer {
    pub const fn new() -> Writer {
        Writer {
            column_position: 0,
            color_code: color_code(Color::LightGray, Color::Black),
            buffer: VGA_BUFFER,
        }
    }

    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.new_line();
            }
            b => {
                if self.column_position >= VGA_WIDTH {
                    self.new_line();
                }
                let row = VGA_HEIGHT - 1;
                let col = self.column_position;
                let v = ((self.color_code as u16) << 8) | b as u16;
                unsafe {
                    core::ptr::write_volatile(self.buffer.add(row * VGA_WIDTH + col), v);
                }
                self.column_position += 1;
            }
        }
    }

    fn new_line(&mut self) {
        // simple scroll up by one line
        unsafe {
            for row in 1..VGA_HEIGHT {
                for col in 0..VGA_WIDTH {
                    let val = core::ptr::read_volatile(self.buffer.add(row * VGA_WIDTH + col));
                    core::ptr::write_volatile(self.buffer.add((row - 1) * VGA_WIDTH + col), val);
                }
            }
            // clear last line
            for col in 0..VGA_WIDTH {
                core::ptr::write_volatile(self.buffer.add((VGA_HEIGHT - 1) * VGA_WIDTH + col), (self.color_code as u16) << 8 | b' ' as u16);
            }
        }
        self.column_position = 0;
    }

    pub fn clear_screen(&mut self) {
        for row in 0..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                unsafe {
                    core::ptr::write_volatile(self.buffer.add(row * VGA_WIDTH + col), (self.color_code as u16) << 8 | b' ' as u16);
                }
            }
        }
        self.column_position = 0;
    }

    pub fn set_color(&mut self, fg: Color, bg: Color) {
        self.color_code = color_code(fg, bg);
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(b'?'),
            }
        }
    }
}

static WRITER: Mutex<Writer> = Mutex::new(Writer::new());

pub fn init() {
    WRITER.lock().clear_screen();
}

pub fn set_color(fg: Color, bg: Color) {
    WRITER.lock().set_color(fg, bg);
}

pub fn print(s: &str) {
    WRITER.lock().write_string(s);
}

// Put a single character to the screen (helper for shell)
pub fn put_char(c: char) {
    WRITER.lock().write_byte(c as u8);
}

// Handle backspace: move cursor back and clear char
pub fn backspace() {
    let mut w = WRITER.lock();
    if w.column_position > 0 {
        w.column_position -= 1;
        let row = VGA_HEIGHT - 1;
        let col = w.column_position;
        unsafe { core::ptr::write_volatile(w.buffer.add(row * VGA_WIDTH + col), (w.color_code as u16) << 8 | b' ' as u16); }
    }
}

pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}

// Implement core::fmt::Write for convenience
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// Macros
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::drivers::vga_buffer::_print(core::format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!(""));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}
