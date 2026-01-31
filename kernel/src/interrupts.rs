//! Interrupt Descriptor Table (IDT) implementation for ospabOS

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::println;
use crate::drivers::keyboard::keyboard as keyboard_getter;
use spin::{Mutex, Once};
use x86_64::instructions::port::Port;

pub struct SimplePics {
    pub offset1: u8,
    pub offset2: u8,
}

impl SimplePics {
    pub const fn new(offset1: u8, offset2: u8) -> Self {
        SimplePics { offset1, offset2 }
    }

    pub fn initialize(&mut self) {
        unsafe {
            let mut pic1 = Port::new(0x20);
            let mut pic2 = Port::new(0xA0);
            // ICW1
            pic1.write(0x11u8);
            pic2.write(0x11u8);
            // ICW2: vector offsets
            pic1.write(self.offset1);
            pic2.write(self.offset2);
            // ICW3
            pic1.write(4u8);
            pic2.write(2u8);
            // ICW4
            pic1.write(0x1u8);
            pic2.write(0x1u8);
            // Mask all interrupts (optional)
            pic1.write(0u8);
            pic2.write(0u8);
        }
    }

    pub fn notify_end_of_interrupt(&mut self, irq: u8) {
        unsafe {
            if irq >= 8 {
                let mut pic2 = Port::new(0xA0);
                pic2.write(0x20u8);
            }
            let mut pic1 = Port::new(0x20);
            pic1.write(0x20u8);
        }
    }
}

static PICS_ONCE: Once<Mutex<SimplePics>> = Once::new();

pub fn pics() -> &'static Mutex<SimplePics> {
    PICS_ONCE.call_once(|| Mutex::new(SimplePics::new(0x20, 0x28)))
}

static IDT_ONCE: Once<InterruptDescriptorTable> = Once::new();

pub fn init_idt() {
    let idt = IDT_ONCE.call_once(|| {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(crate::gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[InterruptIndex::Keyboard.as_usize()]
            .set_handler_fn(keyboard_interrupt_handler);
        idt
    });
    // After initialization, load the IDT
    idt.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: Breakpoint\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: Double Fault\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    if let Some(mut keyboard) = keyboard_getter().try_lock() {
        keyboard.handle_interrupt();
    }
    crate::interrupts::pics().lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
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