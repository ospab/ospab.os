use core::arch::asm;

const IDT_ENTRIES: usize = 256;

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    zero: u32,
}

impl IdtEntry {
    pub const fn new() -> Self {
        IdtEntry {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            zero: 0,
        }
    }

    pub fn set_handler(&mut self, handler: unsafe extern "C" fn(), selector: u16, type_attr: u8) {
        let addr = handler as u64;
        self.offset_low = addr as u16;
        self.offset_mid = (addr >> 16) as u16;
        self.offset_high = (addr >> 32) as u32;
        self.selector = selector;
        self.type_attr = type_attr;
        self.ist = 0;
        self.zero = 0;
    }
}

#[repr(C, packed)]
pub struct IdtDescriptor {
    size: u16,
    offset: u64,
}

pub static mut IDT: [IdtEntry; IDT_ENTRIES] = [IdtEntry::new(); IDT_ENTRIES];

pub fn init_idt() {
    unsafe {
        // Set up handlers for exceptions
        IDT[0].set_handler(division_by_zero_handler, 0x08, 0x8E); // Interrupt gate
        IDT[1].set_handler(debug_handler, 0x08, 0x8E);
        // ... set others
        IDT[14].set_handler(page_fault_handler, 0x08, 0x8E);

        let idt_desc = IdtDescriptor {
            size: (core::mem::size_of::<[IdtEntry; IDT_ENTRIES]>() - 1) as u16,
            offset: &raw const IDT as *const _ as u64,
        };

        asm!("lidt [{}]", in(reg) &idt_desc);
    }
}

extern "C" fn division_by_zero_handler() {
    panic!("Division by zero");
}

extern "C" fn debug_handler() {
    // Handle debug
}

extern "C" fn page_fault_handler() {
    // Handle page fault
    unsafe {
        asm!("cli");
        // Get error code, etc.
        loop {}
    }
}