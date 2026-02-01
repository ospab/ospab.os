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

    // Initialize framebuffer console (no clear yet)
    serial_print(b"[INIT] Initializing framebuffer console... ");
    let fb_ok = drivers::framebuffer::init();
    if fb_ok {
        serial_print(b"OK\r\n");
    } else {
        serial_print(b"FAILED\r\n");
    }

    // Now safe to clear screen (IDT is set up)
    if fb_ok {
        serial_print(b"[INIT] Testing framebuffer writes...\r\n");
        if let Some(fb) = boot::framebuffer() {
            let color: u32 = 0xFFFF0000; // Red
            let ptr = fb.address as *mut u32;
            
            // Use write_volatile for all framebuffer writes
            serial_print(b"       Writing pixel 0... ");
            unsafe { core::ptr::write_volatile(ptr, color); }
            serial_print(b"OK\r\n");
            
            serial_print(b"       Writing pixel 1... ");
            unsafe { core::ptr::write_volatile(ptr.add(1), color); }
            serial_print(b"OK\r\n");
            
            serial_print(b"       Writing pixel 2... ");
            unsafe { core::ptr::write_volatile(ptr.add(2), color); }
            serial_print(b"OK\r\n");
            
            serial_print(b"       Writing pixel 3... ");
            unsafe { core::ptr::write_volatile(ptr.add(3), color); }
            serial_print(b"OK\r\n");
            
            serial_print(b"       Writing pixel 4... ");
            unsafe { core::ptr::write_volatile(ptr.add(4), color); }
            serial_print(b"OK\r\n");
            
            serial_print(b"       Writing pixel 5... ");
            unsafe { core::ptr::write_volatile(ptr.add(5), color); }
            serial_print(b"OK\r\n");
        }
        serial_print(b"[INIT] Test complete!\r\n");
    }

    // Display boot log on screen
    if fb_ok {
        fb_println!("========================================");
        fb_println!("         ospabOS Kernel v0.1.0         ");
        fb_println!("========================================");
        fb_println!();
        fb_println!("[OK] GDT (Global Descriptor Table)");
        fb_println!("[OK] IDT (Interrupt Descriptor Table)");
        fb_println!("[OK] PIC (Programmable Interrupt Controller)");
        fb_println!("[OK] Framebuffer console");
    }

    // Memory info
    serial_print(b"[INFO] Checking memory map...\r\n");
    if let Some(memmap) = boot::memory_map() {
        serial_print(b"[INFO] Memory map found, counting...\r\n");
        let mut usable: u64 = 0;
        for entry in memmap.iter() {
            if entry.typ == 0 { // Usable
                usable += entry.length;
            }
        }
        serial_print(b"[INFO] Usable memory: ");
        serial_print_dec(usable / 1024 / 1024);
        serial_print(b" MB\r\n");
    } else {
        serial_print(b"[INFO] Memory map not available\r\n");
    }

    serial_print(b"[INIT] Kernel initialization complete!\r\n");
    serial_print(b"[INIT] Entering halt loop...\r\n");

    // Enable interrupts
    serial_print(b"[INIT] Enabling interrupts... ");
    x86_64::instructions::interrupts::enable();
    serial_print(b"OK\r\n");
    serial_print(b"\r\n[READY] System initialized, entering main loop\r\n");

    // Main loop
    loop {
        x86_64::instructions::hlt();
    }
}

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