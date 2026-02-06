//! Power management for ospabOS
//! Provides shutdown and reboot functionality

use x86_64::instructions::port::Port;

/// Shutdown the system using ACPI
pub fn shutdown() {
    // Print shutdown message
    crate::drivers::framebuffer::print("\n=== System Shutdown ===\n");
    crate::drivers::framebuffer::print("Shutting down ospabOS...\n");
    
    // Small delay to show message
    for _ in 0..10000000 {
        core::hint::spin_loop();
    }
    
    // Method 1: QEMU/Bochs specific ACPI shutdown port
    // This works in QEMU and Bochs
    unsafe {
        let mut port: Port<u16> = Port::new(0x604);
        port.write(0x2000);
    }
    
    // Method 2: Alternative ACPI shutdown (older systems)
    unsafe {
        let mut port: Port<u16> = Port::new(0xB004);
        port.write(0x2000);
    }
    
    // Method 3: Triple fault (last resort)
    // This will work on any x86 system
    crate::drivers::framebuffer::print("Shutdown failed, initiating triple fault...\n");
    unsafe {
        // Load invalid IDT to trigger triple fault
        #[repr(C, packed)]
        struct InvalidIDT {
            limit: u16,
            base: u64,
        }
        let invalid_idt = InvalidIDT { limit: 0, base: 0 };
        core::arch::asm!(
            "lidt [{}]",
            in(reg) &invalid_idt as *const _ as u64,
            options(nostack)
        );
        // Trigger interrupt with invalid IDT
        core::arch::asm!("int3", options(nostack));
    }
    
    // Should never reach here
    loop {
        x86_64::instructions::hlt();
    }
}

/// Reboot the system using keyboard controller
pub fn reboot() {
    crate::drivers::framebuffer::print("\n=== System Reboot ===\n");
    crate::drivers::framebuffer::print("Rebooting ospabOS...\n");
    
    // Small delay
    for _ in 0..10000000 {
        core::hint::spin_loop();
    }
    
    // Use keyboard controller to pulse reset line
    unsafe {
        let mut port: Port<u8> = Port::new(0x64);
        port.write(0xFE);
    }
    
    // Should never reach here
    loop {
        x86_64::instructions::hlt();
    }
}
