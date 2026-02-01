//! Interrupt Descriptor Table (IDT) implementation for ospabOS
//! Production-ready: uses spin::Lazy, no static mut

use spin::Lazy;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use x86_64::instructions::port::Port;

pub const PIC1_OFFSET: u8 = 0x20;
pub const PIC2_OFFSET: u8 = 0x28;

// ============================================================================
// MINIMAL SERIAL OUTPUT - No dependencies, works in any context
// ============================================================================

/// Write a single byte to serial port with wait
#[inline(always)]
fn serial_byte(b: u8) {
    unsafe {
        let mut port: Port<u8> = Port::new(0x3F8);
        let mut status: Port<u8> = Port::new(0x3FD);
        // Wait for transmit buffer empty (bit 5)
        for _ in 0..10000 {
            if (status.read() & 0x20) != 0 {
                break;
            }
        }
        port.write(b);
    }
}

/// Write string to serial
fn serial_str(s: &[u8]) {
    for &b in s {
        serial_byte(b);
    }
}

/// Write hex value to serial
fn serial_hex(val: u64) {
    const HEX: &[u8] = b"0123456789ABCDEF";
    serial_str(b"0x");
    for i in (0..16).rev() {
        serial_byte(HEX[((val >> (i * 4)) & 0xF) as usize]);
    }
}

// ============================================================================
// HALT FUNCTION - Never returns
// ============================================================================

/// Halt the CPU forever - used after panic
#[inline(never)]
fn halt_forever() -> ! {
    x86_64::instructions::interrupts::disable();
    loop {
        x86_64::instructions::hlt();
    }
}

// ============================================================================
// PANIC SCREEN - Draw red screen with panic info
// ============================================================================

fn draw_panic_screen() {
    if let Some(fb) = crate::boot::framebuffer() {
        let addr = fb.address as *mut u32;
        let width = fb.width as usize;
        let height = fb.height as usize;
        let pitch = fb.pitch as usize / 4;
        
        // Fill screen with dark red
        unsafe {
            for y in 0..height {
                for x in 0..width {
                    let ptr = addr.add(y * pitch + x);
                    core::ptr::write_volatile(ptr, 0xFF8B0000);
                }
            }
            
            // Draw white rectangle in center
            let cx = width / 2;
            let cy = height / 2;
            for y in cy.saturating_sub(40)..core::cmp::min(cy + 40, height) {
                for x in cx.saturating_sub(150)..core::cmp::min(cx + 150, width) {
                    let ptr = addr.add(y * pitch + x);
                    core::ptr::write_volatile(ptr, 0xFFFFFFFF);
                }
            }
        }
    }
}

// ============================================================================
// PIC INITIALIZATION
// ============================================================================

fn initialize_pics() {
    serial_str(b"[PIC] Initializing PICs...\r\n");
    
    unsafe {
        let mut wait_port: Port<u8> = Port::new(0x80);
        let mut pic1_cmd: Port<u8> = Port::new(0x20);
        let mut pic1_data: Port<u8> = Port::new(0x21);
        let mut pic2_cmd: Port<u8> = Port::new(0xA0);
        let mut pic2_data: Port<u8> = Port::new(0xA1);
        
        // ICW1: start initialization sequence
        pic1_cmd.write(0x11);
        wait_port.write(0);
        pic2_cmd.write(0x11);
        wait_port.write(0);
        
        // ICW2: set vector offsets
        pic1_data.write(PIC1_OFFSET); // IRQ 0-7 -> INT 0x20-0x27
        wait_port.write(0);
        pic2_data.write(PIC2_OFFSET); // IRQ 8-15 -> INT 0x28-0x2F
        wait_port.write(0);
        
        // ICW3: cascading setup
        pic1_data.write(4); // IRQ2 has slave
        wait_port.write(0);
        pic2_data.write(2); // Slave ID 2
        wait_port.write(0);
        
        // ICW4: 8086 mode
        pic1_data.write(0x01);
        wait_port.write(0);
        pic2_data.write(0x01);
        wait_port.write(0);
        
        // Mask ALL interrupts except IRQ2 (cascade) for safety
        pic1_data.write(0xFB); // 11111011 - IRQ2 enabled for cascade
        pic2_data.write(0xFF); // All masked on PIC2
        
        serial_str(b"[PIC] PICs initialized, IRQ2 (cascade) enabled\r\n");
    }
}

/// Enable specific IRQs after initialization
pub fn enable_irq(irq: u8) {
    unsafe {
        if irq < 8 {
            let mut pic1_data: Port<u8> = Port::new(0x21);
            let mask = pic1_data.read();
            pic1_data.write(mask & !(1 << irq));
            serial_str(b"[PIC] Enabled IRQ ");
            serial_byte(b'0' + irq);
            serial_str(b"\r\n");
        } else {
            let mut pic2_data: Port<u8> = Port::new(0xA1);
            let mask = pic2_data.read();
            pic2_data.write(mask & !(1 << (irq - 8)));
            // Also enable IRQ2 (cascade)
            let mut pic1_data: Port<u8> = Port::new(0x21);
            let mask1 = pic1_data.read();
            pic1_data.write(mask1 & !(1 << 2));
        }
    }
}

pub fn notify_end_of_interrupt(irq: u8) {
    unsafe {
        if irq >= 8 {
            let mut pic2: Port<u8> = Port::new(0xA0);
            pic2.write(0x20);
        }
        let mut pic1: Port<u8> = Port::new(0x20);
        pic1.write(0x20);
    }
}

// ============================================================================
// IDT SETUP - Using Lazy (no static mut)
// ============================================================================

/// Lazy-initialized IDT with all handlers configured
static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    
    // CPU Exceptions (0-31)
    idt.divide_error.set_handler_fn(divide_error_handler);
    idt.debug.set_handler_fn(debug_handler);
    idt.non_maskable_interrupt.set_handler_fn(nmi_handler);
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    idt.overflow.set_handler_fn(overflow_handler);
    idt.bound_range_exceeded.set_handler_fn(bound_range_handler);
    idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
    idt.device_not_available.set_handler_fn(device_not_available_handler);
    
    // Double fault with separate stack (IST)
    unsafe {
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(crate::gdt::DOUBLE_FAULT_IST_INDEX);
    }
    
    idt.invalid_tss.set_handler_fn(invalid_tss_handler);
    idt.segment_not_present.set_handler_fn(segment_not_present_handler);
    idt.stack_segment_fault.set_handler_fn(stack_segment_handler);
    idt.general_protection_fault.set_handler_fn(gpf_handler);
    idt.page_fault.set_handler_fn(page_fault_handler);
    idt.x87_floating_point.set_handler_fn(x87_fpu_handler);
    idt.alignment_check.set_handler_fn(alignment_check_handler);
    idt.simd_floating_point.set_handler_fn(simd_handler);
    idt.machine_check.set_handler_fn(machine_check_handler);
    idt[20].set_handler_fn(virtualization_exception_handler);
    
    // Hardware interrupts (32+)
    idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
    idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
    
    idt
});

/// Timer tick counter (atomic for interrupt-safety)
use core::sync::atomic::{AtomicU64, Ordering};
static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);

/// Get current tick count (interrupt-safe)
pub fn get_ticks() -> u64 {
    TIMER_TICKS.load(Ordering::Relaxed)
}

pub fn init_idt() {
    serial_str(b"[IDT] Setting up exception handlers...\r\n");
    
    // Force lazy initialization and load IDT
    IDT.load();
    serial_str(b"[IDT] IDT loaded successfully\r\n");
    
    serial_str(b"[IDT] Setting up hardware interrupt handlers...\r\n");
    
    serial_str(b"[IDT] Loading IDT...\r\n");
    serial_str(b"[IDT] IDT loaded successfully\r\n");
    
    // Initialize PICs AFTER IDT is loaded
    initialize_pics();
    
    serial_str(b"[IDT] Initialization complete\r\n");
}

// ============================================================================
// EXCEPTION HANDLERS - All use diverging functions (-> !)
// ============================================================================

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n!!! EXCEPTION: DIVIDE BY ZERO (#DE) !!!\r\n");
    print_stack_frame(&stack_frame);
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n!!! EXCEPTION: DEBUG (#DB) !!!\r\n");
    print_stack_frame(&stack_frame);
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn nmi_handler(stack_frame: InterruptStackFrame) {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n!!! EXCEPTION: NMI !!!\r\n");
    print_stack_frame(&stack_frame);
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    // Breakpoint is recoverable - just log and return
    serial_str(b"\r\n[DEBUG] Breakpoint at ");
    serial_hex(stack_frame.instruction_pointer.as_u64());
    serial_str(b"\r\n");
    // Don't halt - breakpoint is recoverable
}

extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n!!! EXCEPTION: OVERFLOW (#OF) !!!\r\n");
    print_stack_frame(&stack_frame);
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn bound_range_handler(stack_frame: InterruptStackFrame) {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n!!! EXCEPTION: BOUND RANGE EXCEEDED (#BR) !!!\r\n");
    print_stack_frame(&stack_frame);
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n!!! EXCEPTION: INVALID OPCODE (#UD) !!!\r\n");
    serial_str(b"This usually means corrupted code or wrong jump target\r\n");
    print_stack_frame(&stack_frame);
    print_control_registers();
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n!!! EXCEPTION: DEVICE NOT AVAILABLE (#NM) !!!\r\n");
    print_stack_frame(&stack_frame);
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n");
    serial_str(b"################################################################################\r\n");
    serial_str(b"#               !!! DOUBLE FAULT - CRITICAL ERROR !!!                         #\r\n");
    serial_str(b"################################################################################\r\n");
    serial_str(b"Error code: ");
    serial_hex(error_code);
    serial_str(b"\r\n");
    print_stack_frame(&stack_frame);
    print_control_registers();
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n!!! EXCEPTION: INVALID TSS (#TS) !!!\r\n");
    serial_str(b"Error code: ");
    serial_hex(error_code);
    serial_str(b"\r\n");
    print_stack_frame(&stack_frame);
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn segment_not_present_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n!!! EXCEPTION: SEGMENT NOT PRESENT (#NP) !!!\r\n");
    serial_str(b"Error code: ");
    serial_hex(error_code);
    serial_str(b"\r\n");
    print_stack_frame(&stack_frame);
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn stack_segment_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n!!! EXCEPTION: STACK SEGMENT FAULT (#SS) !!!\r\n");
    serial_str(b"Error code: ");
    serial_hex(error_code);
    serial_str(b"\r\n");
    print_stack_frame(&stack_frame);
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn gpf_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n!!! EXCEPTION: GENERAL PROTECTION FAULT (#GP) !!!\r\n");
    serial_str(b"Error code: ");
    serial_hex(error_code);
    serial_str(b"\r\n");
    serial_str(b"This usually means: invalid segment, privilege violation, or bad memory access\r\n");
    print_stack_frame(&stack_frame);
    print_control_registers();
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    x86_64::instructions::interrupts::disable();
    
    let cr2 = x86_64::registers::control::Cr2::read_raw();
    
    serial_str(b"\r\n!!! EXCEPTION: PAGE FAULT (#PF) !!!\r\n");
    serial_str(b"Faulting address (CR2): ");
    serial_hex(cr2);
    serial_str(b"\r\n");
    serial_str(b"Error code: ");
    serial_hex(error_code.bits());
    serial_str(b"\r\n");
    
    serial_str(b"Cause: ");
    if error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
        serial_str(b"PROTECTION_VIOLATION ");
    } else {
        serial_str(b"PAGE_NOT_PRESENT ");
    }
    if error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE) {
        serial_str(b"(WRITE) ");
    } else {
        serial_str(b"(READ) ");
    }
    if error_code.contains(PageFaultErrorCode::USER_MODE) {
        serial_str(b"USER_MODE ");
    }
    if error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH) {
        serial_str(b"INSTRUCTION_FETCH ");
    }
    serial_str(b"\r\n");
    
    print_stack_frame(&stack_frame);
    print_control_registers();
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn x87_fpu_handler(stack_frame: InterruptStackFrame) {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n!!! EXCEPTION: x87 FPU ERROR (#MF) !!!\r\n");
    print_stack_frame(&stack_frame);
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn alignment_check_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n!!! EXCEPTION: ALIGNMENT CHECK (#AC) !!!\r\n");
    serial_str(b"Error code: ");
    serial_hex(error_code);
    serial_str(b"\r\n");
    print_stack_frame(&stack_frame);
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn simd_handler(stack_frame: InterruptStackFrame) {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n!!! EXCEPTION: SIMD FLOATING POINT (#XF) !!!\r\n");
    print_stack_frame(&stack_frame);
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) -> ! {
    x86_64::instructions::interrupts::disable();
    serial_str(b"\r\n");
    serial_str(b"################################################################################\r\n");
    serial_str(b"#             !!! MACHINE CHECK EXCEPTION (#MC) !!!                           #\r\n");
    serial_str(b"################################################################################\r\n");
    serial_str(b"VMware/Hardware reported critical error\r\n");
    print_stack_frame(&stack_frame);
    print_control_registers();
    draw_panic_screen();
    halt_forever();
}

extern "x86-interrupt" fn virtualization_exception_handler(stack_frame: InterruptStackFrame) {
    // Virtualization Exception - just log and return
    serial_str(b"\r\n[WARN] Virtualization Exception (#VE) at ");
    serial_hex(stack_frame.instruction_pointer.as_u64());
    serial_str(b"\r\n");
    // Recoverable - just return
}

// ============================================================================
// HARDWARE INTERRUPT HANDLERS
// ============================================================================

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Minimal handler - just send EOI
    unsafe {
        core::arch::asm!(
            "mov al, 0x20",
            "out 0x20, al",
            options(nomem, nostack, preserves_flags)
        );
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Read status register first to check if data is available
    let status: u8 = unsafe {
        let mut port = Port::<u8>::new(0x64);
        port.read()
    };
    
    // Check if output buffer is full (data available)
    if (status & 0x01) == 0 {
        // Spurious interrupt - acknowledge and return
        notify_end_of_interrupt(1);
        return;
    }
    
    // Read scancode from keyboard data port
    let scancode: u8 = unsafe {
        let mut port = Port::<u8>::new(0x60);
        port.read()
    };
    
    // Queue for processing in main loop
    crate::drivers::keyboard::queue_scancode(scancode);
    
    // Acknowledge interrupt to PIC
    notify_end_of_interrupt(1);
}

// ============================================================================
// DEBUG HELPERS
// ============================================================================

fn print_stack_frame(sf: &InterruptStackFrame) {
    serial_str(b"\r\n=== Stack Frame ===\r\n");
    serial_str(b"RIP: ");
    serial_hex(sf.instruction_pointer.as_u64());
    serial_str(b"\r\n");
    serial_str(b"RSP: ");
    serial_hex(sf.stack_pointer.as_u64());
    serial_str(b"\r\n");
    serial_str(b"RFLAGS: ");
    serial_hex(sf.cpu_flags);
    serial_str(b"\r\n");
    serial_str(b"CS: ");
    serial_hex(sf.code_segment);
    serial_str(b"\r\n");
    serial_str(b"SS: ");
    serial_hex(sf.stack_segment);
    serial_str(b"\r\n");
}

fn print_control_registers() {
    use x86_64::registers::control::{Cr0, Cr2, Cr3, Cr4};
    
    serial_str(b"\r\n=== Control Registers ===\r\n");
    serial_str(b"CR0: ");
    serial_hex(Cr0::read_raw());
    serial_str(b"\r\n");
    serial_str(b"CR2: ");
    serial_hex(Cr2::read_raw());
    serial_str(b"\r\n");
    serial_str(b"CR3: ");
    serial_hex(Cr3::read_raw().0.start_address().as_u64());
    serial_str(b"\r\n");
    serial_str(b"CR4: ");
    serial_hex(Cr4::read_raw());
    serial_str(b"\r\n");
}

// ============================================================================
// INTERRUPT INDEX ENUM
// ============================================================================

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = 32,    // PIC1_OFFSET + 0
    Keyboard = 33, // PIC1_OFFSET + 1
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}
