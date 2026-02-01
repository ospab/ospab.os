// Hardware abstraction layers
// VGA driver

pub mod vga_buffer;
pub mod keyboard;
pub mod framebuffer;

const VGA_BUFFER: *mut u16 = 0xB8000 as *mut u16;
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;

pub struct VgaDriver;

impl VgaDriver {
    pub fn write_char(x: usize, y: usize, c: u8, color: u8) {
        if x < VGA_WIDTH && y < VGA_HEIGHT {
            unsafe {
                *VGA_BUFFER.add(y * VGA_WIDTH + x) = (color as u16) << 8 | c as u16;
            }
        }
    }

    pub fn clear_screen() {
        for y in 0..VGA_HEIGHT {
            for x in 0..VGA_WIDTH {
                Self::write_char(x, y, b' ', 0x07);
            }
        }
    }

    pub fn print_str(x: usize, y: usize, s: &str, color: u8) {
        let mut cx = x;
        for byte in s.bytes() {
            if cx >= VGA_WIDTH { break; }
            Self::write_char(cx, y, byte, color);
            cx += 1;
        }
    }
}