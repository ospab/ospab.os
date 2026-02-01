//! Interrupt Descriptor Table (IDT) implementation for ospabOS

#![allow(static_mut_refs)]

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use x86_64::instructions::port::Port;

pub const PIC1_OFFSET: u8 = 0x20;
pub const PIC2_OFFSET: u8 = 0x28;

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

extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {
    serial_halt(b"BREAKPOINT");
}

extern "x86-interrupt" fn divide_error_handler(_stack_frame: InterruptStackFrame) {
    serial_halt(b"DIVIDE ERROR");
}

extern "x86-interrupt" fn debug_handler(_stack_frame: InterruptStackFrame) {
    serial_halt(b"DEBUG");
}

extern "x86-interrupt" fn nmi_handler(_stack_frame: InterruptStackFrame) {
    serial_halt(b"NMI");
}

extern "x86-interrupt" fn overflow_handler(_stack_frame: InterruptStackFrame) {
    serial_halt(b"OVERFLOW");
}

extern "x86-interrupt" fn bound_range_handler(_stack_frame: InterruptStackFrame) {
    serial_halt(b"BOUND RANGE");
}

extern "x86-interrupt" fn invalid_opcode_handler(_stack_frame: InterruptStackFrame) {
    serial_halt(b"INVALID OPCODE");
}

extern "x86-interrupt" fn device_not_available_handler(_stack_frame: InterruptStackFrame) {
    serial_halt(b"DEVICE NOT AVAILABLE");
}

extern "x86-interrupt" fn double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial_halt(b"DOUBLE FAULT");
    loop { x86_64::instructions::hlt(); }
}

extern "x86-interrupt" fn invalid_tss_handler(_stack_frame: InterruptStackFrame, _error_code: u64) {
    serial_halt(b"INVALID TSS");
}

extern "x86-interrupt" fn segment_not_present_handler(_stack_frame: InterruptStackFrame, _error_code: u64) {
    serial_halt(b"SEGMENT NOT PRESENT");
}

extern "x86-interrupt" fn stack_segment_handler(_stack_frame: InterruptStackFrame, _error_code: u64) {
    serial_halt(b"STACK SEGMENT FAULT");
}

extern "x86-interrupt" fn gpf_handler(_stack_frame: InterruptStackFrame, _error_code: u64) {
    serial_halt(b"GENERAL PROTECTION FAULT");
}

extern "x86-interrupt" fn page_fault_handler(_stack_frame: InterruptStackFrame, _error_code: x86_64::structures::idt::PageFaultErrorCode) {
    serial_halt(b"PAGE FAULT");
}

extern "x86-interrupt" fn x87_fpu_handler(_stack_frame: InterruptStackFrame) {
    serial_halt(b"X87 FPU ERROR");
}

extern "x86-interrupt" fn alignment_check_handler(_stack_frame: InterruptStackFrame, _error_code: u64) {
    serial_halt(b"ALIGNMENT CHECK");
}

extern "x86-interrupt" fn simd_handler(_stack_frame: InterruptStackFrame) {
    serial_halt(b"SIMD ERROR");
}

fn serial_halt(msg: &[u8]) {
    unsafe {
        use x86_64::instructions::port::Port;
        let mut port = Port::<u8>::new(0x3F8);
        for &b in b"\r\n!!! EXCEPTION: " { port.write(b); }
        for &b in msg { port.write(b); }
        for &b in b" !!!\r\n" { port.write(b); }
    }
    loop { x86_64::instructions::hlt(); }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // MINIMAL HANDLER: Just read scancode and ACK (don't process)
    // This prevents keyboard buffer overflow
    let _scancode: u8 = unsafe {
        let mut port = x86_64::instructions::port::Port::<u8>::new(0x60);
        port.read()
    };
    
    // Don't process - just acknowledge and discard
    // crate::drivers::keyboard::queue_scancode(scancode); // DISABLED
    
    // IRQ 1 (keyboard)
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