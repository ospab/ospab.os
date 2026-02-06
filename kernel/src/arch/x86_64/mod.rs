use core::arch::asm;
use x86_64::structures::paging::{PageTable, OffsetPageTable};
use x86_64::VirtAddr;

pub fn init() {
    init_paging();
    // Enable SSE
    unsafe {
        asm!(
            "mov rax, cr0",
            "and ax, 0xFFFB",
            "or ax, 0x2",
            "mov cr0, rax",
            "mov rax, cr4",
            "or ax, 3 << 9",
            "mov cr4, rax"
        );
    }
}

fn init_paging() {
    // Do not create global identity mappings here — the bootloader already provides
    // the identity/high mappings necessary for early boot. Creating massive
    // identity mappings (0..4GB) causes conflicts (PageAlreadyMapped) and is slow.
    serial_write_str("paging: no-op\n");
}

#[allow(dead_code)]
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let (level_4_table_frame, _) = x86_64::registers::control::Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    OffsetPageTable::new(&mut *page_table_ptr, physical_memory_offset)
}

pub fn halt() {
    unsafe { asm!("hlt") };
}

pub fn enable_interrupts() {
    unsafe { asm!("sti") };
}

pub fn disable_interrupts() {
    unsafe { asm!("cli") };
}

pub fn outb(port: u16, value: u8) {
    unsafe { asm!("out dx, al", in("dx") port, in("al") value) };
}

pub fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe { asm!("in al, dx", out("al") value, in("dx") port) };
    value
}

// Simple polled serial (COM1 at 0x3F8) helpers for early debug
pub fn serial_init() {
    // Disable interrupts
    outb(0x3F8 + 1, 0x00);
    // Enable DLAB (set baud rate divisor)
    outb(0x3F8 + 3, 0x80);
    // Set divisor to 3 (lo byte) 38400 baud
    outb(0x3F8 + 0, 0x03);
    //                  (hi byte)
    outb(0x3F8 + 1, 0x00);
    // 8 bits, no parity, one stop bit
    outb(0x3F8 + 3, 0x03);
    // Enable FIFO, clear them, with 14-byte threshold
    outb(0x3F8 + 2, 0xC7);
    // IRQs enabled, RTS/DSR set
    outb(0x3F8 + 4, 0x0B);
}

pub fn serial_write_byte(byte: u8) {
    // Wait for transmit buffer to be empty (bit 5 of line status)
    while (inb(0x3F8 + 5) & 0x20) == 0 {}
    outb(0x3F8, byte);
}

pub fn serial_write_str(s: &str) {
    for &b in s.as_bytes() {
        serial_write_byte(b);
    }
}

// QEMU/Bochs debug port (0xE9) write — often visible on host console
pub fn debug_port_write(byte: u8) {
    outb(0xE9, byte);
}

pub fn debug_port_write_str(s: &str) {
    for &b in s.as_bytes() {
        debug_port_write(b);
    }
}

const USER_TRANSITION_STACK_SIZE: usize = 4096 * 4;

#[repr(C, align(16))]
struct UserTransitionStack {
    data: [u8; USER_TRANSITION_STACK_SIZE],
}

static USER_TRANSITION_STACK: UserTransitionStack = UserTransitionStack {
    data: [0; USER_TRANSITION_STACK_SIZE],
};

pub unsafe fn enter_user_mode(entry: u64, user_stack: u64) -> ! {
    let selectors = crate::gdt::selectors();
    let user_code = (selectors.user_code.0 | 3) as u64;
    let user_data = (selectors.user_data.0 | 3) as u64;

    asm!(
        "mov ds, r8w",
        "mov es, r8w",
        "mov fs, r8w",
        "mov gs, r8w",
        "push r8",
        "push r9",
        "pushfq",
        "pop rax",
        "or rax, 0x200",
        "push rax",
        "push r10",
        "push r11",
        "iretq",
        in("r8") user_data,
        in("r9") user_stack,
        in("r10") user_code,
        in("r11") entry,
        options(noreturn)
    )
}

pub unsafe fn enter_user_mode_with_cr3(entry: u64, user_stack: u64, cr3: u64) -> ! {
    let selectors = crate::gdt::selectors();
    let user_code = (selectors.user_code.0 | 3) as u64;
    let user_data = (selectors.user_data.0 | 3) as u64;

    asm!(
        "lea rsp, [rip + {stack_base}]",
        "add rsp, {stack_size}",
        "mov cr3, {cr3}",
        "mov r8, {data}",
        "mov r9, {ustack}",
        "mov r10, {code}",
        "mov r11, {entry}",
        "mov ds, r8w",
        "mov es, r8w",
        "mov fs, r8w",
        "mov gs, r8w",
        "push r8",
        "push r9",
        "pushfq",
        "pop rax",
        "or rax, 0x200",
        "push rax",
        "push r10",
        "push r11",
        "iretq",
        stack_base = sym USER_TRANSITION_STACK,
        stack_size = const USER_TRANSITION_STACK_SIZE,
        cr3 = in(reg) cr3,
        data = in(reg) user_data,
        ustack = in(reg) user_stack,
        code = in(reg) user_code,
        entry = in(reg) entry,
        options(noreturn)
    )
}