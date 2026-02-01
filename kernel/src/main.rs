//! ospabOS Kernel Entry Point
//! A minimal but stable kernel with proper interrupt handling

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

extern crate alloc;
extern crate ospab_os;

use core::panic::PanicInfo;
use ospab_os::{boot, drivers, fb_println, gdt, interrupts, mm, process};

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
// PANIC HANDLER - Full debug output to Serial (COM1)
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
    
    // Dump control registers
    serial_print(b"\r\n=== Control Registers ===\r\n");
    unsafe {
        let cr0: u64;
        let cr2: u64;
        let cr3: u64;
        let cr4: u64;
        core::arch::asm!("mov {}, cr0", out(reg) cr0);
        core::arch::asm!("mov {}, cr2", out(reg) cr2);
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
        core::arch::asm!("mov {}, cr4", out(reg) cr4);
        
        serial_print(b"CR0: ");
        serial_hex(cr0);
        serial_print(b"\r\n");
        serial_print(b"CR2: ");
        serial_hex(cr2);
        serial_print(b"\r\n");
        serial_print(b"CR3: ");
        serial_hex(cr3);
        serial_print(b"\r\n");
        serial_print(b"CR4: ");
        serial_hex(cr4);
        serial_print(b"\r\n");
    }
    
    // Dump RSP
    serial_print(b"\r\n=== Stack ===\r\n");
    unsafe {
        let rsp: u64;
        core::arch::asm!("mov {}, rsp", out(reg) rsp);
        serial_print(b"RSP: ");
        serial_hex(rsp);
        serial_print(b"\r\n");
    }
    
    serial_print(b"\r\nSystem halted. Power off manually.\r\n");
    
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
    
    // Enable SSE/SSE2 - required for x86-interrupt calling convention
    unsafe {
        // Set CR4.OSFXSR and CR4.OSXMMEXCPT
        core::arch::asm!(
            "mov rax, cr4",
            "or rax, 0x600",  // bits 9 (OSFXSR) and 10 (OSXMMEXCPT)
            "mov cr4, rax",
            options(nostack, preserves_flags)
        );
        // Clear CR0.EM, set CR0.MP
        core::arch::asm!(
            "mov rax, cr0",
            "and ax, 0xFFFB",  // clear EM (bit 2)
            "or ax, 0x2",      // set MP (bit 1)
            "mov cr0, rax",
            options(nostack, preserves_flags)
        );
    }
    
    serial_print(b"\r\n");
    serial_print(b"========================================\r\n");
    serial_print(b"         ospabOS Kernel v0.1.0         \r\n");
    serial_print(b"========================================\r\n");
    
    // Step 1: Verify bootloader
    serial_print(b"[1/7] Checking Limine protocol... ");
    serial_print(b"rev=");
    serial_hex(boot::get_base_revision_raw());
    serial_print(b" ");
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
    serial_print(b"[3/7] Initializing GDT...\r\n");
    gdt::init();
    serial_print(b"[3/7] GDT loaded successfully\r\n");
    
    // Step 4: Initialize IDT and PICs
    serial_print(b"[4/7] Initializing IDT and PICs...\r\n");
    interrupts::init_idt();
    serial_print(b"[4/7] IDT and PICs ready\r\n");
    
    // Step 5: Initialize framebuffer
    serial_print(b"[5/7] Initializing framebuffer...\r\n");
    let fb_ok = drivers::framebuffer::init();
    if fb_ok {
        serial_print(b"[5/7] Framebuffer OK\r\n");
        if let Some(fb) = boot::framebuffer() {
            serial_print(b"       Resolution: ");
            serial_hex(fb.width);
            serial_print(b" x ");
            serial_hex(fb.height);
            serial_print(b"\r\n");
        }
    } else {
        serial_print(b"[5/7] Framebuffer FAILED\r\n");
    }
    
    // Step 6: Initialize keyboard driver (no interrupts yet)
    serial_print(b"[6/7] Initializing keyboard driver...\r\n");
    drivers::keyboard::init();
    serial_print(b"[6/7] Keyboard driver ready\r\n");
    
    // Step 7: System ready
    serial_print(b"[7/7] All components initialized\r\n");
    
    serial_print(b"\r\n");
    serial_print(b"========================================\r\n");
    serial_print(b"     All systems initialized!          \r\n");
    serial_print(b"========================================\r\n");
    
    // === LINUX-LIKE SUBSYSTEMS ===
    serial_print(b"\r\n[SUBSYS] Initializing kernel subsystems...\r\n");
    
    // Memory management
    serial_print(b"[SUBSYS] Initializing memory management...\r\n");
    mm::init();
    
    // Timer (PIT)
    serial_print(b"[SUBSYS] Initializing timer (PIT)...\r\n");
    drivers::timer::init();
    interrupts::enable_irq(0); // Enable timer interrupt
    
    // Process management
    serial_print(b"[SUBSYS] Initializing process management...\r\n");
    process::init();
    
    serial_print(b"[SUBSYS] All subsystems online\r\n");
    
    serial_print(b"\r\n[FB] Preparing screen output...\r\n");
    // Display welcome on screen
    if fb_ok {
        serial_print(b"[FB] Drawing welcome screen...\r\n");
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
        serial_print(b"[FB] Welcome screen drawn\r\n");
    } else {
        serial_print(b"[FB] Skipped - framebuffer not available\r\n");
    }
    
    // === CRITICAL SEQUENCE FOR VMWARE ===
    // Step 1: Enable CPU interrupts (sti)
    serial_print(b"\r\n[INIT] Enabling CPU interrupts (sti)...\r\n");
    x86_64::instructions::interrupts::enable();
    serial_print(b"[INIT] CPU interrupts enabled!\r\n");
    
    // Tiny delay - system should be stable immediately
    for _ in 0..100 {
        core::hint::spin_loop();
    }
    serial_print(b"[INIT] System stable after sti\r\n");
    
    // Step 2: Enable keyboard IRQ (AFTER sti, at the very end)
    serial_print(b"[INIT] Enabling keyboard hardware IRQ...\r\n");
    drivers::keyboard::enable_hw_irq();
    serial_print(b"[INIT] Keyboard IRQ enabled!\r\n");
    
    serial_print(b"\r\n[FB] Drawing prompt...\r\n");
    if fb_ok {
        fb_println!("[OK] Interrupts enabled");
        fb_println!();
        fb_println!("Ready. Type 'help' for commands.");
        fb_println!();
        drivers::framebuffer::print("[ospab]~> ");
        drivers::framebuffer::show_cursor();
        serial_print(b"[FB] Prompt drawn, cursor shown\r\n");
    } else {
        serial_print(b"[FB] Skipped - framebuffer not available\r\n");
    }
    
    serial_print(b"\r\n[READY] Entering main loop\r\n");
    
    let mut tick_counter: u64 = 0;
    
    // Main event loop (Linux-style with scheduler)
    loop {
        // Process keyboard events
        drivers::keyboard::process_scancodes();
        
        // Check timer ticks
        let current_jiffies = drivers::timer::get_jiffies();
        if current_jiffies != tick_counter {
            tick_counter = current_jiffies;
            
            // Every second, print uptime
            if tick_counter % 100 == 0 {
                serial_print(b"[UPTIME] ");
                serial_hex(drivers::timer::get_uptime_ms() / 1000);
                serial_print(b" seconds\r\n");
            }
        }
        
        // Small pause to reduce CPU usage
        core::hint::spin_loop();
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
