//! Interrupt Descriptor Table (IDT) implementation for ospabOS

#![allow(static_mut_refs)]

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use x86_64::instructions::port::Port;

pub const PIC1_OFFSET: u8 = 0x20;
pub const PIC2_OFFSET: u8 = 0x28;

// ============================================================================
// PANIC INFRASTRUCTURE - Uses only serial port, no locks, no allocations
// ============================================================================

/// Write a single byte to serial port - absolutely minimal, no dependencies
#[inline(always)]
fn serial_byte(b: u8) {
    unsafe {
        let mut port: Port<u8> = Port::new(0x3F8);
        // Wait for transmit buffer empty
        let mut status: Port<u8> = Port::new(0x3FD);
        while (status.read() & 0x20) == 0 {}
        port.write(b);
    }
}

/// Write string to serial - no dependencies
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

/// KERNEL PANIC - Never returns, never reboots
/// Uses only serial port for output (safe in any context)
fn kernel_panic(exception: &[u8], stack_frame: &InterruptStackFrame, error_code: Option<u64>) -> ! {
    // Disable ALL interrupts immediately
    x86_64::instructions::interrupts::disable();
    
    serial_str(b"\r\n");
    serial_str(b"################################################################################\r\n");
    serial_str(b"#                                                                              #\r\n");
    serial_str(b"#                         !!! KERNEL PANIC !!!                                 #\r\n");
    serial_str(b"#                                                                              #\r\n");
    serial_str(b"################################################################################\r\n");
    serial_str(b"\r\n");
    
    serial_str(b"Exception: ");
    serial_str(exception);
    serial_str(b"\r\n\r\n");
    
    if let Some(code) = error_code {
        serial_str(b"Error Code: ");
        serial_hex(code);
        serial_str(b"\r\n\r\n");
    }
    
    serial_str(b"=== CPU State ===\r\n");
    serial_str(b"RIP (Instruction Pointer): ");
    serial_hex(stack_frame.instruction_pointer.as_u64());
    serial_str(b"\r\n");
    
    serial_str(b"RSP (Stack Pointer):       ");
    serial_hex(stack_frame.stack_pointer.as_u64());
    serial_str(b"\r\n");
    
    serial_str(b"RFLAGS:                    ");
    // cpu_flags is already u64
    serial_hex(stack_frame.cpu_flags);
    serial_str(b"\r\n");
    
    serial_str(b"CS (Code Segment):         ");
    // code_segment is u64 in newer x86_64 versions
    serial_hex(stack_frame.code_segment);
    serial_str(b"\r\n");
    
    serial_str(b"SS (Stack Segment):        ");
    serial_hex(stack_frame.stack_segment);
    serial_str(b"\r\n\r\n");
    
    serial_str(b"=== Control Registers ===\r\n");
    use x86_64::registers::control::{Cr0, Cr2, Cr3, Cr4};
    serial_str(b"CR0: ");
    serial_hex(Cr0::read_raw());
    serial_str(b"\r\n");
    serial_str(b"CR2 (Page Fault Addr): ");
    // Cr2::read_raw() returns u64 directly
    serial_hex(Cr2::read_raw());
    serial_str(b"\r\n");
    serial_str(b"CR3 (Page Table Base): ");
    serial_hex(Cr3::read_raw().0.start_address().as_u64());
    serial_str(b"\r\n");
    serial_str(b"CR4: ");
    serial_hex(Cr4::read_raw());
    serial_str(b"\r\n\r\n");
    
    serial_str(b"################################################################################\r\n");
    serial_str(b"#                       SYSTEM HALTED - NO REBOOT                              #\r\n");
    serial_str(b"#                    Please power off manually                                 #\r\n");
    serial_str(b"################################################################################\r\n");
    
    // Try to draw panic on framebuffer (may fail, but won't cause issues)
    draw_panic_screen(exception);
    
    // HALT FOREVER - Never reboot
    loop {
        x86_64::instructions::hlt();
    }
}

/// Try to draw panic on screen - if framebuffer is available
/// This is best-effort, failure is acceptable
fn draw_panic_screen(exception: &[u8]) {
    // Direct framebuffer write - bypass all abstractions
    if let Some(fb) = crate::boot::framebuffer() {
        let addr = fb.address as *mut u32;
        let width = fb.width as usize;
        let height = fb.height as usize;
        let pitch = fb.pitch as usize / 4; // pitch in u32s
        
        // Fill screen with red
        unsafe {
            for y in 0..height {
                for x in 0..width {
                    let ptr = addr.add(y * pitch + x);
                    core::ptr::write_volatile(ptr, 0xFFAA0000); // Red
                }
            }
        }
        
        // Draw white text "KERNEL PANIC" at center (simple, no font needed)
        // Just draw a white rectangle as indicator
        let cx = width / 2;
        let cy = height / 2;
        unsafe {
            for y in (cy - 50)..(cy + 50) {
                for x in (cx - 200)..(cx + 200) {
                    if y < height && x < width {
                        let ptr = addr.add(y * pitch + x);
                        core::ptr::write_volatile(ptr, 0xFFFFFFFF); // White
                    }
                }
            }
        }
        
        let _ = exception; // Used for serial only
    }
}

fn initialize_pics() {
    unsafe {
        let mut wait_port: Port<u8> = Port::new(0x80);
        let mut pic1_cmd: Port<u8> = Port::new(0x20);
        let mut pic1_data: Port<u8> = Port::new(0x21);
        let mut pic2_cmd: Port<u8> = Port::new(0xA0);
        let mut pic2_data: Port<u8> = Port::new(0xA1);
        
        // Save masks (not used but good practice)
        let _mask1 = pic1_data.read();
        let _mask2 = pic2_data.read();
        
        // ICW1: start initialization
        pic1_cmd.write(0x11);
        wait_port.write(0);
        pic2_cmd.write(0x11);
        wait_port.write(0);
        
        // ICW2: vector offsets
        pic1_data.write(PIC1_OFFSET);
        wait_port.write(0);
        pic2_data.write(PIC2_OFFSET);
        wait_port.write(0);
        
        // ICW3: cascading
        pic1_data.write(4); // slave on IRQ2
        wait_port.write(0);
        pic2_data.write(2); // cascade identity
        wait_port.write(0);
        
        // ICW4: 8086 mode
        pic1_data.write(0x01);
        wait_port.write(0);
        pic2_data.write(0x01);
        wait_port.write(0);
        
        // Unmask timer (IRQ0) and keyboard (IRQ1)
        // Mask = 0 means enabled, 1 means disabled
        pic1_data.write(0b11111100); // Enable IRQ0 (timer) and IRQ1 (keyboard)
        pic2_data.write(0b11111111); // Mask all on PIC2
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

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();
static mut IDT_INITIALIZED: bool = false;

pub fn init_idt() {
    unsafe {
        if IDT_INITIALIZED {
            return;
        }
        
        // Initialize PICs first
        initialize_pics();
        
        // Setup exception handlers (CPU exceptions 0-31)
        IDT.divide_error.set_handler_fn(divide_error_handler);
        IDT.debug.set_handler_fn(debug_handler);
        IDT.non_maskable_interrupt.set_handler_fn(nmi_handler);
        IDT.breakpoint.set_handler_fn(breakpoint_handler);
        IDT.overflow.set_handler_fn(overflow_handler);
        IDT.bound_range_exceeded.set_handler_fn(bound_range_handler);
        IDT.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        IDT.device_not_available.set_handler_fn(device_not_available_handler);
        IDT.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(crate::gdt::DOUBLE_FAULT_IST_INDEX);
        IDT.invalid_tss.set_handler_fn(invalid_tss_handler);
        IDT.segment_not_present.set_handler_fn(segment_not_present_handler);
        IDT.stack_segment_fault.set_handler_fn(stack_segment_handler);
        IDT.general_protection_fault.set_handler_fn(gpf_handler);
        IDT.page_fault.set_handler_fn(page_fault_handler);
        IDT.x87_floating_point.set_handler_fn(x87_fpu_handler);
        IDT.alignment_check.set_handler_fn(alignment_check_handler);
        IDT.simd_floating_point.set_handler_fn(simd_handler);
        
        // Hardware interrupts
        IDT[InterruptIndex::Timer.as_usize()]
            .set_handler_fn(timer_interrupt_handler);
        IDT[InterruptIndex::Keyboard.as_usize()]
            .set_handler_fn(keyboard_interrupt_handler);
        
        // Load the IDT
        IDT.load();
        
        IDT_INITIALIZED = true;
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    kernel_panic(b"BREAKPOINT (#BP)", &stack_frame, None);
}

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    kernel_panic(b"DIVIDE BY ZERO (#DE)", &stack_frame, None);
}

extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    kernel_panic(b"DEBUG EXCEPTION (#DB)", &stack_frame, None);
}

extern "x86-interrupt" fn nmi_handler(stack_frame: InterruptStackFrame) {
    kernel_panic(b"NON-MASKABLE INTERRUPT (NMI)", &stack_frame, None);
}

extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    kernel_panic(b"OVERFLOW (#OF)", &stack_frame, None);
}

extern "x86-interrupt" fn bound_range_handler(stack_frame: InterruptStackFrame) {
    kernel_panic(b"BOUND RANGE EXCEEDED (#BR)", &stack_frame, None);
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    kernel_panic(b"INVALID OPCODE (#UD)", &stack_frame, None);
}

extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    kernel_panic(b"DEVICE NOT AVAILABLE (#NM)", &stack_frame, None);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    kernel_panic(b"DOUBLE FAULT (#DF) - CRITICAL", &stack_frame, Some(error_code));
}

extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    kernel_panic(b"INVALID TSS (#TS)", &stack_frame, Some(error_code));
}

extern "x86-interrupt" fn segment_not_present_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    kernel_panic(b"SEGMENT NOT PRESENT (#NP)", &stack_frame, Some(error_code));
}

extern "x86-interrupt" fn stack_segment_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    kernel_panic(b"STACK SEGMENT FAULT (#SS)", &stack_frame, Some(error_code));
}

extern "x86-interrupt" fn gpf_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    kernel_panic(b"GENERAL PROTECTION FAULT (#GP)", &stack_frame, Some(error_code));
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    // Get the faulting address from CR2
    let cr2 = x86_64::registers::control::Cr2::read_raw();
    
    serial_str(b"\r\n!!! PAGE FAULT !!!\r\n");
    serial_str(b"Faulting Address (CR2): ");
    serial_hex(cr2);
    serial_str(b"\r\n");
    serial_str(b"Error Code Bits: ");
    serial_hex(error_code.bits());
    serial_str(b"\r\n");
    
    // Decode error code
    serial_str(b"  - ");
    if error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
        serial_str(b"PROTECTION_VIOLATION ");
    } else {
        serial_str(b"PAGE_NOT_PRESENT ");
    }
    if error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE) {
        serial_str(b"WRITE ");
    } else {
        serial_str(b"READ ");
    }
    if error_code.contains(PageFaultErrorCode::USER_MODE) {
        serial_str(b"USER_MODE ");
    } else {
        serial_str(b"KERNEL_MODE ");
    }
    if error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH) {
        serial_str(b"INSTRUCTION_FETCH ");
    }
    serial_str(b"\r\n");
    
    kernel_panic(b"PAGE FAULT (#PF)", &stack_frame, Some(error_code.bits()));
}

extern "x86-interrupt" fn x87_fpu_handler(stack_frame: InterruptStackFrame) {
    kernel_panic(b"x87 FPU ERROR (#MF)", &stack_frame, None);
}

extern "x86-interrupt" fn alignment_check_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    kernel_panic(b"ALIGNMENT CHECK (#AC)", &stack_frame, Some(error_code));
}

extern "x86-interrupt" fn simd_handler(stack_frame: InterruptStackFrame) {
    kernel_panic(b"SIMD FLOATING POINT (#XM/#XF)", &stack_frame, None);
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Read scancode from keyboard controller
    let scancode: u8 = unsafe {
        let mut port = x86_64::instructions::port::Port::<u8>::new(0x60);
        port.read()
    };
    
    // Queue scancode for processing in main loop
    crate::drivers::keyboard::queue_scancode(scancode);
    
    // Send EOI to PIC (IRQ 1 = keyboard)
    notify_end_of_interrupt(1);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Just acknowledge the interrupt, don't do anything else
    // IRQ 0 (timer)
    notify_end_of_interrupt(0);
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = 32,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

#[cfg(test)]
mod tests {
    #[test_case]
    fn test_breakpoint_exception() {
        // Invoke a breakpoint exception
        x86_64::instructions::interrupts::int3();
    }
}