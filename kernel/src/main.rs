//! ospabOS Kernel Entry Point
//! A minimal but stable kernel with proper interrupt handling

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(c_variadic)]

extern crate alloc;
extern crate ospab_os;

use core::panic::PanicInfo;
use ospab_os::{boot, drivers, fb_println, gdt, interrupts, mm, process, ipc, services, shell, task, mem, syscall, auth, net};

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
    serial_print(b"       ospabOS v0.1.0 \"Foundation\"     \r\n");
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
    
    // Step 6: Initialize serial port for hardware debugging
    serial_print(b"[6/8] Initializing serial port (COM1)...\r\n");
    drivers::serial::init();
    serial_print(b"[6/8] Serial port ready\r\n");
    
    // Step 7: Initialize keyboard driver (no interrupts yet)
    serial_print(b"[7/8] Initializing keyboard driver...\r\n");
    drivers::keyboard::init();
    serial_print(b"[7/8] Keyboard driver ready\r\n");
    
    // Step 8: System ready
    serial_print(b"[8/8] All components initialized\r\n");
    
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
    
    // === v0.1.0 "FOUNDATION" INITIALIZATION ===
    serial_print(b"\r\n[v0.1.0] Initializing Foundation components...\r\n");
    
    // Task management with TSS
    serial_print(b"[v0.1.0] Initializing task management (TSS + scheduler)...\r\n");
    task::init();
    
    // Frame allocator
    serial_print(b"[v0.1.0] Initializing frame allocator...\r\n");
    mem::physical::FRAME_ALLOCATOR.lock().init(0x100000, 0x200000); // Kernel at 1MB-2MB
    
    // Virtual Memory Manager (v0.1.5)
    serial_print(b"[v0.1.5] Initializing Virtual Memory Manager...\r\n");
    if let Err(e) = mem::init_vmm() {
        serial_print(b"[ERROR] Failed to initialize VMM: ");
        serial_print(e.as_bytes());
        serial_print(b"\r\n");
    } else {
        serial_print(b"[v0.1.5] VMM initialized successfully\r\n");
    }
    
    // Syscall interface (v0.1.5)
    serial_print(b"[v0.1.5] Initializing syscall interface...\r\n");
    syscall::init();
    serial_print(b"[v0.1.5] Syscall interface ready\r\n");
    
    serial_print(b"[v0.1.0] Foundation components initialized\r\n");
    
    // === MICROKERNEL IPC ARCHITECTURE ===
    serial_print(b"\r\n[IPC] Initializing microkernel services...\r\n");
    
    // Message Bus
    serial_print(b"[IPC] Initializing message bus...\r\n");
    ipc::bus::init();
    
    // Terminal Service (wraps existing I/O)
    serial_print(b"[IPC] Initializing terminal service...\r\n");
    services::terminal::init();
    
    // VFS Service
    serial_print(b"[IPC] Initializing VFS service...\r\n");
    services::vfs::init();

    // User Authentication System
    serial_print(b"[AUTH] Initializing user authentication...\r\n");
    auth::init();

    // Network Stack
    serial_print(b"[NET] Initializing network stack...\r\n");
    net::init();

    
    serial_print(b"\r\n[FB] Preparing screen output...\r\n");
    // Display welcome on screen
    if fb_ok {
        serial_print(b"[FB] Drawing welcome screen...\r\n");
        fb_println!("========================================");
        fb_println!("  ospabOS v0.1.0 \"Foundation\"");
        fb_println!("  Preemptive Multitasking + Syscalls");
        fb_println!("========================================");
        fb_println!();
        fb_println!("[OK] GDT initialized");
        fb_println!("[OK] IDT initialized");
        fb_println!("[OK] Task Scheduler (Round-Robin)");
        fb_println!("[OK] TSS configured");
        fb_println!("[OK] Frame Allocator ready");
        fb_println!("[OK] Memory Map parsed (USABLE regions)");
        fb_println!("[OK] UEFI Framebuffer (RGB/BGR auto)");
        fb_println!("[OK] Serial port (COM1) ready");
        fb_println!("[OK] Keyboard driver loaded");
        fb_println!("[OK] IPC Message Bus ready");
        fb_println!("[OK] Terminal Service online");
        fb_println!("[OK] VFS Service (Initrd) online");
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
        fb_println!("Message-passing microkernel architecture");
        fb_println!("Type 'help' for commands. Try: ls, cat test.txt");
        fb_println!();
        
        // Show prompt with current directory
        let prompt = shell::get_prompt();
        drivers::framebuffer::print(&prompt);
        drivers::framebuffer::show_cursor();
        serial_print(b"[FB] Prompt drawn, cursor shown\r\n");
    } else {
        serial_print(b"[FB] Skipped - framebuffer not available\r\n");
    }
    
    serial_print(b"\r\n[READY] Entering main loop\r\n");
    
    let mut tick_counter: u64 = 0;
    
    // Main event loop - microkernel message processing
    loop {
        // Process keyboard events (Terminal Service)
        services::terminal::poll_input();
        
        // Check timer ticks
        let current_jiffies = drivers::timer::get_jiffies();
        if current_jiffies != tick_counter {
            tick_counter = current_jiffies;
            
            // Blink cursor every 50 ticks (500ms)
            if tick_counter % 50 == 0 {
                drivers::framebuffer::toggle_cursor();
            }
        }
        
        // Halt CPU until next interrupt (saves power and allows interrupts to fire)
        x86_64::instructions::hlt();
    }
}

// ============================================================================
// PROGRESS BAR FOR BOOT LOADING - TEMPORARILY DISABLED
// ============================================================================

/*
fn draw_progress_bar(step: usize, total: usize, message: &str) {
    if !drivers::framebuffer::is_initialized() {
        return;
    }
    
    let bar_width = 40;
    let filled = (step * bar_width) / total;
    let percent = (step * 100) / total;
    
    // Clear previous progress area (assume lines 20-25)
    for y in 20..26 {
        for x in 0..80 {
            drivers::framebuffer::set_pixel(x * 8, y * 16, 0x00000000);
        }
    }
    
    // Draw message
    drivers::framebuffer::set_cursor(0, 20);
    drivers::framebuffer::print(message);
    
    // Draw progress bar
    drivers::framebuffer::set_cursor(0, 22);
    drivers::framebuffer::print("[");
    for i in 0..bar_width {
        if i < filled {
            drivers::framebuffer::print("█");
        } else {
            drivers::framebuffer::print("░");
        }
    }
    drivers::framebuffer::print("]");
    
    // Draw percentage
    drivers::framebuffer::set_cursor(0, 24);
    drivers::framebuffer::print(&alloc::format!("{}% complete", percent));
}
*/

fn halt_forever() -> ! {
    serial_print(b"FATAL: System halted\r\n");
    loop {
        x86_64::instructions::hlt();
    }
}

// ============================================================================
// KERNEL ENTRY POINT
// ============================================================================
