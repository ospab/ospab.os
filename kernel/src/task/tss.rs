//! Task State Segment (TSS) for ospabOS v0.1.0
//! Provides privilege-level stack switching

use alloc::format;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;
use spin::Mutex;

/// Global TSS instance
static TSS: Mutex<TaskStateSegment> = Mutex::new(TaskStateSegment::new());

/// Kernel stack for interrupts (separate from task stacks)
const INTERRUPT_STACK_SIZE: usize = 4096 * 5; // 20 KB
static mut INTERRUPT_STACK: [u8; INTERRUPT_STACK_SIZE] = [0; INTERRUPT_STACK_SIZE];

/// Initialize TSS
pub fn init() {
    let mut tss = TSS.lock();
    
    // Set up interrupt stack (IST1)
    let stack_ptr = core::ptr::addr_of!(INTERRUPT_STACK);
    let stack_top = VirtAddr::from_ptr(stack_ptr) + INTERRUPT_STACK_SIZE;
    tss.interrupt_stack_table[0] = stack_top;
    
    // Set up privilege stack 0 (kernel)
    tss.privilege_stack_table[0] = stack_top;
    
    // Load TSS into GDT (will be done by GDT module)
    crate::serial_println!("[TSS] Initialized with interrupt stack at {:#x}", stack_top.as_u64());
}

/// Get TSS reference for GDT
pub fn get_tss() -> &'static TaskStateSegment {
    unsafe { &*(TSS.lock().deref() as *const _) }
}

/// Set kernel stack for current task (called during context switch)
pub fn set_kernel_stack(stack_top: VirtAddr) {
    TSS.lock().privilege_stack_table[0] = stack_top;
}

use core::ops::Deref;
