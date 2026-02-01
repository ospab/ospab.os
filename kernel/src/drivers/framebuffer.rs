//! Framebuffer-based console driver for ospabOS
//! Uses Limine's framebuffer for graphical text output

use crate::boot;
use spin::Mutex;

/// Simple 8x8 bitmap font (embedded)
/// Each character is 8 bytes (8 rows of 8 pixels)
static FONT_8X8: [u8; 760] = include!("font_data.rs");

pub struct FramebufferConsole {
    fb_addr: *mut u8,
    width: usize,
    height: usize,
    pitch: usize,
    bpp: usize,
    
    // Console state
    cursor_x: usize,
    cursor_y: usize,
    char_width: usize,
    char_height: usize,
    cols: usize,
    rows: usize,
    
    // Colors (32-bit BGRA)
    fg_color: u32,
    bg_color: u32,
    
    // Cursor blinking
    cursor_visible: bool,
}

unsafe impl Send for FramebufferConsole {}

impl FramebufferConsole {
    pub const fn empty() -> Self {
        FramebufferConsole {
            fb_addr: core::ptr::null_mut(),
            width: 0,
            height: 0,
            pitch: 0,
            bpp: 0,
            cursor_x: 0,
            cursor_y: 0,
            char_width: 12,   // 8x8 font scaled 1.5x
            char_height: 12,
            cols: 0,
            rows: 0,
            fg_color: 0x00FFFFFF, // White
            bg_color: 0x00000000, // Black
            cursor_visible: true,
        }
    }
    
    pub fn init_from_limine(&mut self) -> bool {
        if let Some(fb) = boot::framebuffer() {
            self.fb_addr = fb.address;
            self.width = fb.width as usize;
            self.height = fb.height as usize;
            self.pitch = fb.pitch as usize;
            self.bpp = fb.bpp as usize / 8;
            
            self.cols = self.width / self.char_width;
            self.rows = self.height / self.char_height;
            
            // DON'T clear screen during init - may cause triple fault
            // Let the caller do it after IDT is set up
            true
        } else {
            false
        }
    }
    
    pub fn is_initialized(&self) -> bool {
        !self.fb_addr.is_null()
    }
    
    pub fn set_colors(&mut self, fg: u32, bg: u32) {
        self.fg_color = fg;
        self.bg_color = bg;
    }
    
    pub fn clear(&mut self) {
        if self.fb_addr.is_null() || self.width == 0 || self.height == 0 {
            return;
        }
        
        unsafe {
            let color = self.bg_color | 0xFF000000;
            // Use pitch correctly - pitch is in bytes
            for y in 0..self.height {
                let row_ptr = self.fb_addr.add(y * self.pitch) as *mut u32;
                for x in 0..self.width {
                    core::ptr::write_volatile(row_ptr.add(x), color);
                }
            }
        }
        
        self.cursor_x = 0;
        self.cursor_y = 0;
    }
    
    #[inline]
    unsafe fn put_pixel(&self, x: usize, y: usize, color: u32) {
        // Strict bounds checking for VMware compatibility
        if x >= self.width || y >= self.height {
            return;
        }
        if self.fb_addr.is_null() {
            return;
        }
        
        let offset = y * self.pitch + x * self.bpp;
        let ptr = self.fb_addr.add(offset) as *mut u32;
        
        // Write as 32-bit value using write_volatile
        core::ptr::write_volatile(ptr, color | 0xFF000000);
    }
    
    fn draw_char(&self, x: usize, y: usize, c: char) {
        if self.fb_addr.is_null() {
            return;
        }
        
        let c = c as usize;
        if c < 32 || c > 126 {
            return;
        }
        
        let font_index = (c - 32) * 8; // 8 bytes per character (8x8 font)
        
        // Draw with simple nearest-neighbor scaling to 12x12
        for py in 0..self.char_height {
            let row = py * 8 / self.char_height; // Map to 0-7
            let font_byte = if font_index + row < FONT_8X8.len() {
                FONT_8X8[font_index + row]
            } else {
                0
            };
            
            for px in 0..self.char_width {
                let col = px * 8 / self.char_width; // Map to 0-7
                let color = if (font_byte >> (7 - col)) & 1 == 1 {
                    self.fg_color
                } else {
                    self.bg_color
                };
                
                unsafe {
                    self.put_pixel(x + px, y + py, color);
                }
            }
        }
    }
    
    fn scroll(&mut self) {
        if self.fb_addr.is_null() {
            return;
        }
        
        // Copy all lines up by one
        unsafe {
            let line_bytes = self.pitch * self.char_height;
            let total_lines = self.rows - 1;
            
            for line in 0..total_lines {
                let src = self.fb_addr.add((line + 1) * self.char_height * self.pitch);
                let dst = self.fb_addr.add(line * self.char_height * self.pitch);
                core::ptr::copy(src, dst, line_bytes);
            }
            
            // Clear the last line
            for y in 0..self.char_height {
                for x in 0..self.width {
                    self.put_pixel(x, (self.rows - 1) * self.char_height + y, self.bg_color);
                }
            }
        }
    }
    
    pub fn write_char(&mut self, c: char) {
        if self.fb_addr.is_null() {
            return;
        }
        
        match c {
            '\n' => {
                self.cursor_x = 0;
                self.cursor_y += 1;
                if self.cursor_y >= self.rows {
                    self.cursor_y = self.rows - 1;
                    self.scroll();
                }
            }
            '\r' => {
                self.cursor_x = 0;
            }
            '\x08' => {
                // Backspace
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                    self.draw_char(
                        self.cursor_x * self.char_width,
                        self.cursor_y * self.char_height,
                        ' ',
                    );
                }
            }
            c => {
                self.draw_char(
                    self.cursor_x * self.char_width,
                    self.cursor_y * self.char_height,
                    c,
                );
                self.cursor_x += 1;
                if self.cursor_x >= self.cols {
                    self.cursor_x = 0;
                    self.cursor_y += 1;
                    if self.cursor_y >= self.rows {
                        self.cursor_y = self.rows - 1;
                        self.scroll();
                    }
                }
            }
        }
    }
    
    pub fn write_str(&mut self, s: &str) {
        for c in s.chars() {
            self.write_char(c);
        }
    }
    
    pub fn cols(&self) -> usize {
        self.cols
    }
    
    pub fn rows(&self) -> usize {
        self.rows
    }
    
    /// Draw cursor at current position
    pub fn draw_cursor(&self, visible: bool) {
        if self.fb_addr.is_null() {
            return;
        }
        
        let x = self.cursor_x * self.char_width;
        let y = self.cursor_y * self.char_height;
        
        // Draw underscore cursor
        let cursor_y_start = y + self.char_height - 2; // Bottom 2 pixels
        let color = if visible { self.fg_color } else { self.bg_color };
        
        for py in 0..2 {
            for px in 0..self.char_width {
                unsafe {
                    self.put_pixel(x + px, cursor_y_start + py, color);
                }
            }
        }
    }
    
    /// Toggle cursor visibility
    pub fn toggle_cursor(&mut self) {
        self.cursor_visible = !self.cursor_visible;
        self.draw_cursor(self.cursor_visible);
    }
    
    /// Hide cursor before writing
    pub fn hide_cursor(&mut self) {
        if self.cursor_visible {
            self.draw_cursor(false);
        }
    }
    
    /// Show cursor after writing
    pub fn show_cursor(&mut self) {
        self.cursor_visible = true;
        self.draw_cursor(true);
    }
}

static CONSOLE: Mutex<FramebufferConsole> = Mutex::new(FramebufferConsole::empty());

pub fn init() -> bool {
    let mut console = CONSOLE.lock();
    console.init_from_limine()
}

pub fn is_initialized() -> bool {
    // Use try_lock to avoid deadlock in interrupt context
    if let Some(console) = CONSOLE.try_lock() {
        console.is_initialized()
    } else {
        // If locked, assume it's initialized (conservative)
        true
    }
}

pub fn print(s: &str) {
    if let Some(mut console) = CONSOLE.try_lock() {
        console.write_str(s);
    }
}

pub fn print_char(c: char) {
    if let Some(mut console) = CONSOLE.try_lock() {
        console.write_char(c);
    }
}

pub fn clear() {
    if let Some(mut console) = CONSOLE.try_lock() {
        console.clear();
    }
}

/// Alias for clear() - clears the screen
pub fn clear_screen() {
    clear();
}

pub fn set_colors(fg: u32, bg: u32) {
    if let Some(mut console) = CONSOLE.try_lock() {
        console.set_colors(fg, bg);
    }
}

/// Toggle cursor (called from timer interrupt)
pub fn toggle_cursor() {
    if let Some(mut console) = CONSOLE.try_lock() {
        console.toggle_cursor();
    }
}

/// Show cursor
pub fn show_cursor() {
    if let Some(mut console) = CONSOLE.try_lock() {
        console.show_cursor();
    }
}

// Implement fmt::Write
use core::fmt;

pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    CONSOLE.lock().write_fmt(args).unwrap();
}

impl fmt::Write for FramebufferConsole {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! fb_print {
    ($($arg:tt)*) => ($crate::drivers::framebuffer::_print(core::format_args!($($arg)*)));
}

#[macro_export]
macro_rules! fb_println {
    () => ($crate::fb_print!("\n"));
    ($fmt:expr) => ($crate::fb_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::fb_print!(concat!($fmt, "\n"), $($arg)*));
}
