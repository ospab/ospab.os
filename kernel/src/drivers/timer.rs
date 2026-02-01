//! Programmable Interval Timer (PIT) driver
//! Used for preemptive multitasking and timekeeping (like Linux jiffies)

use x86_64::instructions::port::Port;
use core::sync::atomic::{AtomicU64, Ordering};

const PIT_FREQUENCY: u32 = 1193182; // Base PIT frequency
const TARGET_HZ: u32 = 100; // 100 Hz = 10ms per tick

static JIFFIES: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    let divisor = (PIT_FREQUENCY / TARGET_HZ) as u16;
    
    unsafe {
        // Command: Channel 0, rate generator mode, 16-bit counter
        let mut cmd_port: Port<u8> = Port::new(0x43);
        cmd_port.write(0x36);
        
        // Set divisor
        let mut data_port: Port<u8> = Port::new(0x40);
        data_port.write((divisor & 0xFF) as u8);
        data_port.write((divisor >> 8) as u8);
    }
}

/// Called from timer interrupt handler
pub fn tick() {
    JIFFIES.fetch_add(1, Ordering::Relaxed);
}

/// Get current tick count (like Linux jiffies)
pub fn get_jiffies() -> u64 {
    JIFFIES.load(Ordering::Relaxed)
}

/// Get uptime in milliseconds
pub fn get_uptime_ms() -> u64 {
    get_jiffies() * 10 // 10ms per tick
}
