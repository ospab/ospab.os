//! Serial Port (COM1) driver for hardware debugging
//!
//! This driver provides logging to COM1 (0x3F8) for debugging on real hardware
//! where the screen might not work properly.

use x86_64::instructions::port::Port;
use spin::Mutex;
use core::fmt::{self, Write};

const SERIAL_PORT: u16 = 0x3F8; // COM1

pub struct SerialPort {
    data: Port<u8>,
    line_status: Port<u8>,
}

impl SerialPort {
    /// Create a new serial port instance
    const fn new() -> Self {
        SerialPort {
            data: Port::new(SERIAL_PORT),
            line_status: Port::new(SERIAL_PORT + 5),
        }
    }
    
    /// Initialize the serial port
    pub fn init(&mut self) {
        unsafe {
            // Disable interrupts
            Port::new(SERIAL_PORT + 1).write(0x00u8);
            
            // Enable DLAB (set baud rate divisor)
            Port::new(SERIAL_PORT + 3).write(0x80u8);
            
            // Set divisor to 3 (38400 baud)
            Port::new(SERIAL_PORT + 0).write(0x03u8);
            Port::new(SERIAL_PORT + 1).write(0x00u8);
            
            // 8 bits, no parity, one stop bit
            Port::new(SERIAL_PORT + 3).write(0x03u8);
            
            // Enable FIFO, clear with 14-byte threshold
            Port::new(SERIAL_PORT + 2).write(0xC7u8);
            
            // Mark data terminal ready, request to send
            Port::new(SERIAL_PORT + 4).write(0x0Bu8);
        }
    }
    
    /// Check if transmit buffer is empty
    fn is_transmit_empty(&mut self) -> bool {
        unsafe { (self.line_status.read() & 0x20) != 0 }
    }
    
    /// Check if data is available to read
    pub fn is_data_available(&mut self) -> bool {
        unsafe { (self.line_status.read() & 0x01) != 0 }
    }
    
    /// Read a byte from the serial port (non-blocking)
    pub fn read_byte(&mut self) -> Option<u8> {
        if self.is_data_available() {
            Some(unsafe { self.data.read() })
        } else {
            None
        }
    }
    
    /// Send a byte to the serial port
    pub fn send_byte(&mut self, byte: u8) {
        // Wait for transmit buffer to be empty (with timeout)
        for _ in 0..10000 {
            if self.is_transmit_empty() {
                break;
            }
        }
        
        unsafe {
            self.data.write(byte);
        }
    }
    
    /// Write a string to the serial port
    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.send_byte(b'\r');
            }
            self.send_byte(byte);
        }
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

/// Global serial port instance
static SERIAL: Mutex<SerialPort> = Mutex::new(SerialPort::new());

/// Initialize the serial port
pub fn init() {
    SERIAL.lock().init();
}

/// Write string to serial port
pub fn write(s: &str) {
    SERIAL.lock().write_str(s);
}

/// Write formatted string to serial port
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::drivers::serial::write(&format!($($arg)*))
    };
}

/// Write formatted string with newline to serial port
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}

/// Log message with prefix
pub fn log(message: &str) {
    write("[SERIAL] ");
    write(message);
    write("\n");
}

/// Log error message
pub fn error(message: &str) {
    write("[ERROR] ");
    write(message);
    write("\n");
}

/// Log info message
pub fn info(message: &str) {
    write("[INFO] ");
    write(message);
    write("\n");
}

/// Log debug message
pub fn debug(message: &str) {
    write("[DEBUG] ");
    write(message);
    write("\n");
}

/// Poll serial port for input (non-blocking)
pub fn poll_input() -> Option<u8> {
    SERIAL.lock().read_byte()
}
