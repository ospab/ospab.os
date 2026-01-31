use core::arch::asm;

pub fn init_exceptions() {
    // Exceptions are set in IDT
}

pub fn handle_exception(vector: u8, _error_code: u32) {
    match vector {
        0 => panic!("Division by zero"),
        1 => panic!("Debug exception"),
        2 => panic!("Non-maskable interrupt"),
        3 => panic!("Breakpoint"),
        4 => panic!("Overflow"),
        5 => panic!("Bound range exceeded"),
        6 => panic!("Invalid opcode"),
        7 => panic!("Device not available"),
        8 => panic!("Double fault"),
        9 => panic!("Coprocessor segment overrun"),
        10 => panic!("Invalid TSS"),
        11 => panic!("Segment not present"),
        12 => panic!("Stack-segment fault"),
        13 => panic!("General protection fault"),
        14 => {
            // Page fault
            let cr2: u64;
            unsafe { asm!("mov {}, cr2", out(reg) cr2) };
            panic!("Page fault at address {:x}", cr2);
        }
        15 => panic!("Reserved"),
        16 => panic!("x87 FPU floating-point error"),
        17 => panic!("Alignment check"),
        18 => panic!("Machine check"),
        19 => panic!("SIMD floating-point exception"),
        20 => panic!("Virtualization exception"),
        21..=31 => panic!("Reserved"),
        _ => panic!("Unknown exception"),
    }
}