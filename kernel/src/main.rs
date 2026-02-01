//! ospabOS Kernel Entry Point
//! A minimal but stable kernel with proper interrupt handling

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate ospab_os;

use core::panic::PanicInfo;
use ospab_os::{boot, drivers, fb_println, gdt, interrupts};

// ============================================================================
// SERIAL OUTPUT - For debugging
// ============================================================================

fn serial_print(msg: &[u8]) {
    unsafe {
        use x86_64::instructions::port::Port;
        let mut port = Port::<u8>::new(0x3F8);
        let mut status = Port::<u8>::new(0x3FD);
        for &b in msg.iter() {
            // Wait for transmit buffer
            for _ in 0..10000 {
                if (status.read() & 0x20) != 0 {
                    break;
                }
            }
            port.write(b);
        }
    }
}

fn serial_hex(val: u64) {
    let hex_chars = b"0123456789ABCDEF";
    serial_print(b"0x");
    for i in (0..16).rev() {
        let nibble = ((val >> (i * 4)) & 0xF) as usize;
        unsafe {
            use x86_64::instructions::port::Port;
            let mut port = Port::<u8>::new(0x3F8);
            port.write(hex_chars[nibble]);
        }
    }
}

// ============================================================================
// PANIC HANDLER - Never reboots
// ============================================================================

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Disable all interrupts immediately
    x86_64::instructions::interrupts::disable();
    
    serial_print(b"\r\n");
    serial_print(b"################################################################################\r\n");
    serial_print(b"#                         RUST PANIC HANDLER                                  #\r\n");
    serial_print(b"################################################################################\r\n");
    
    // Try to print panic info to serial
    if let Some(location) = info.location() {
        serial_print(b"Location: ");
        serial_print(location.file().as_bytes());
        serial_print(b":");
        serial_hex(location.line() as u64);
        serial_print(b"\r\n");
    }
    
    serial_print(b"System halted. Power off manually.\r\n");
    
    // Try to show on framebuffer
    if drivers::framebuffer::is_initialized() {
        drivers::framebuffer::set_colors(0x00FF0000, 0x00000000);
        drivers::framebuffer::print("\n\n!!! KERNEL PANIC !!!\n");
        drivers::framebuffer::print("System halted.\n");
    }
    
    // Halt forever
    loop {
        x86_64::instructions::hlt();
    }
}

// ============================================================================
// KERNEL ENTRY POINT
// ============================================================================

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // CRITICAL: Disable interrupts until everything is set up
    x86_64::instructions::interrupts::disable();
    
    serial_print(b"\r\n");
    serial_print(b"========================================\r\n");
    serial_print(b"         ospabOS Kernel v0.1.0         \r\n");
    serial_print(b"========================================\r\n");
    
    // Step 1: Verify bootloader
    serial_print(b"[1/7] Checking Limine protocol... ");
    if !boot::base_revision_supported() {
        serial_print(b"FAILED\r\n");
        halt_forever();
    }
    serial_print(b"OK\r\n");
    
    // Step 2: Get HHDM offset
    serial_print(b"[2/7] Getting HHDM offset... ");
    if let Some(offset) = boot::hhdm_offset() {
        serial_hex(offset);
        serial_print(b" OK\r\n");
    } else {
        serial_print(b"NOT AVAILABLE\r\n");
    }
    
    // Step 3: Initialize GDT (MUST be before IDT)
    serial_print(b"[3/7] Initializing GDT... ");
    gdt::init();
    serial_print(b"OK\r\n");
    
    // Step 4: Initialize IDT and PICs
    serial_print(b"[4/7] Initializing IDT and PICs... ");
    interrupts::init_idt();
    serial_print(b"OK\r\n");
    
    // Step 5: Initialize framebuffer
    serial_print(b"[5/7] Initializing framebuffer... ");
    let fb_ok = drivers::framebuffer::init();
    if fb_ok {
        serial_print(b"OK\r\n");
        if let Some(fb) = boot::framebuffer() {
            serial_print(b"       Resolution: ");
            serial_hex(fb.width);
            serial_print(b" x ");
            serial_hex(fb.height);
            serial_print(b"\r\n");
        }
    } else {
        serial_print(b"FAILED\r\n");
    }
    
    // Step 6: Initialize keyboard driver (no interrupts yet)
    serial_print(b"[6/7] Initializing keyboard driver... ");
    drivers::keyboard::init();
    serial_print(b"OK\r\n");
    
    // Step 7: System ready
    serial_print(b"[7/7] All components ready... OK\r\n");
    
    serial_print(b"\r\n");
    serial_print(b"========================================\r\n");
    serial_print(b"     All systems initialized!          \r\n");
    serial_print(b"========================================\r\n");
    
    // Display welcome on screen
    if fb_ok {
        fb_println!("========================================");
        fb_println!("       ospabOS Kernel v0.1.0");
        fb_println!("========================================");
        fb_println!();
        fb_println!("[OK] GDT initialized");
        fb_println!("[OK] IDT initialized");
        fb_println!("[OK] PIC configured");
        fb_println!("[OK] Framebuffer ready");
        fb_println!("[OK] Keyboard driver loaded");
        fb_println!();
    }
    
    // Now enable interrupts for keyboard
    serial_print(b"\r\n[INIT] Enabling keyboard interrupt (IRQ1)...\r\n");
    interrupts::enable_irq(1); // Enable keyboard only
    
    serial_print(b"[INIT] Enabling CPU interrupts (sti)...\r\n");
    x86_64::instructions::interrupts::enable();
    serial_print(b"[INIT] Interrupts enabled successfully!\r\n");
    
    if fb_ok {
        fb_println!("[OK] Interrupts enabled");
        fb_println!();
        fb_println!("Ready. Type 'help' for commands.");
        fb_println!();
        drivers::framebuffer::print("[ospab]~> ");
    }
    
    serial_print(b"\r\n[READY] Entering main loop\r\n");
    
    // Main loop
    loop {
        // Disable interrupts while processing
        x86_64::instructions::interrupts::disable();
        
        // Process any queued keyboard input
        drivers::keyboard::process_scancodes();
        
        // Re-enable and wait for next interrupt
        x86_64::instructions::interrupts::enable();
        x86_64::instructions::hlt();
    }
}

// ============================================================================
// HELPERS
// ============================================================================

fn halt_forever() -> ! {
    serial_print(b"FATAL: System halted\r\n");
    loop {
        x86_64::instructions::hlt();
    }
}
