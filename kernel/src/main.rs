#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate ospab_os;

use core::panic::PanicInfo;
use ospab_os::{boot, drivers, fb_println, gdt, interrupts};

/// Serial port output for debugging
fn serial_print(msg: &[u8]) {
    unsafe {
        use x86_64::instructions::port::Port;
        let mut port = Port::<u8>::new(0x3F8);
        for &b in msg.iter() {
            port.write(b);
        }
    }
}

fn serial_print_hex(val: u64) {
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

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // Disable interrupts to prevent further issues
    x86_64::instructions::interrupts::disable();
    
    serial_print(b"\r\n");
    serial_print(b"========================================\r\n");
    serial_print(b"         !!! KERNEL PANIC !!!          \r\n");
    serial_print(b"========================================\r\n");
    serial_print(b"System halted. Please restart manually.\r\n");
    
    // Try to display on framebuffer if available
    if drivers::framebuffer::is_initialized() {
        drivers::framebuffer::set_colors(0x00FF0000, 0x00000000); // Red on black
        drivers::framebuffer::print("\n\n!!! KERNEL PANIC !!!\n");
        drivers::framebuffer::print("System halted.\n");
    }
    
    // Halt forever - don't reboot
    loop {
        x86_64::instructions::hlt();
    }
}

/// Kernel entry point - called by Limine bootloader
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Disable interrupts until we set up proper handlers
    x86_64::instructions::interrupts::disable();
    
    serial_print(b"\r\n");
    serial_print(b"========================================\r\n");
    serial_print(b"         ospabOS Kernel v0.1.0         \r\n");
    serial_print(b"========================================\r\n");

    // Check Limine base revision
    serial_print(b"[BOOT] Checking Limine protocol... ");
    if !boot::base_revision_supported() {
        serial_print(b"FAILED\r\n");
        serial_print(b"ERROR: Limine base revision not supported!\r\n");
        halt_forever();
    }
    serial_print(b"OK\r\n");

    // Get HHDM offset
    serial_print(b"[BOOT] Getting HHDM offset... ");
    let _hhdm = if let Some(offset) = boot::hhdm_offset() {
        serial_print(b"OK (");
        serial_print_hex(offset);
        serial_print(b")\r\n");
        offset
    } else {
        serial_print(b"NOT AVAILABLE\r\n");
        0
    };

    // *** CRITICAL: Initialize GDT and IDT FIRST to catch faults ***
    serial_print(b"[INIT] Initializing GDT... ");
    gdt::init();
    serial_print(b"OK\r\n");

    serial_print(b"[INIT] Initializing IDT and PICs... ");
    interrupts::init_idt();
    serial_print(b"OK\r\n");

    // Get framebuffer info
    serial_print(b"[BOOT] Checking framebuffer... ");
    if let Some(fb) = boot::framebuffer() {
        serial_print(b"OK\r\n");
        serial_print(b"       Address: ");
        serial_print_hex(fb.address as u64);
        serial_print(b"\r\n");
        serial_print(b"       Width: ");
        serial_print_hex(fb.width);
        serial_print(b"  Height: ");
        serial_print_hex(fb.height);
        serial_print(b"\r\n");
        serial_print(b"       Pitch: ");
        serial_print_hex(fb.pitch);
        serial_print(b"  BPP: ");
        serial_print_hex(fb.bpp as u64);
        serial_print(b"\r\n");
    } else {
        serial_print(b"NOT AVAILABLE\r\n");
    }

    // Initialize framebuffer console
    serial_print(b"[INIT] Initializing framebuffer console... ");
    let fb_ok = drivers::framebuffer::init();
    if fb_ok {
        serial_print(b"OK\r\n");
    } else {
        serial_print(b"FAILED\r\n");
    }

    // Clear screen and show boot info
    if fb_ok {
        // NOTE: clear() is slow, skip for now
        // serial_print(b"[INIT] Clearing screen...\r\n");
        // drivers::framebuffer::clear();
        serial_print(b"[INIT] Printing boot info...\r\n");
        fb_println!("========================================");
        fb_println!("       ospabOS Kernel v0.1.0");
        fb_println!("========================================");
        fb_println!();
        fb_println!("[OK] GDT initialized");
        fb_println!("[OK] IDT initialized");
        fb_println!("[OK] PIC configured");
        fb_println!("[OK] Framebuffer ready");
        fb_println!();
        serial_print(b"[INIT] Boot info printed\r\n");
    }

    // Memory info
    serial_print(b"[INFO] Checking memory map...\r\n");
    if let Some(_memmap) = boot::memory_map() {
        serial_print(b"[INFO] Memory map available\r\n");
        if fb_ok {
            fb_println!("[OK] Memory map available");
        }
    }

    // Initialize keyboard driver
    // DISABLED: Causes triple fault in VMware
    // serial_print(b"[INIT] Initializing keyboard driver... ");
    // drivers::keyboard::init();
    // serial_print(b"OK\r\n");
    // if fb_ok {
    //     fb_println!("[OK] Keyboard driver loaded");
    // }

    serial_print(b"[INIT] Kernel initialization complete!\r\n");

    // Enable interrupts for keyboard
    serial_print(b"[INIT] Enabling interrupts... ");
    x86_64::instructions::interrupts::enable();
    serial_print(b"OK\r\n");

    if fb_ok {
        fb_println!();
        fb_println!("System Ready.");
        fb_println!("Keyboard: Disabled (VMware triple fault)");
        fb_println!("Waiting in idle loop...");
        fb_println!();
    }

    serial_print(b"\r\n[READY] System initialized\r\n");

    // Main loop - just halt (keyboard disabled due to VMware triple fault)
    loop {
        // drivers::keyboard::process_scancodes(); // DISABLED
        x86_64::instructions::hlt();
    }
}

#[allow(dead_code)]
fn serial_print_dec(mut val: u64) {
    if val == 0 {
        serial_print(b"0");
        return;
    }
    let mut buf = [0u8; 20];
    let mut i = 0;
    while val > 0 {
        buf[i] = b'0' + (val % 10) as u8;
        val /= 10;
        i += 1;
    }
    // Reverse and print
    while i > 0 {
        i -= 1;
        unsafe {
            use x86_64::instructions::port::Port;
            let mut port = Port::<u8>::new(0x3F8);
            port.write(buf[i]);
        }
    }
}

fn halt_forever() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}