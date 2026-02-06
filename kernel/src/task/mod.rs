//! Task Management for ospabOS v0.1.0
//! Implements preemptive multitasking with TSS and context switching

use alloc::format;

pub mod pcb;
pub mod scheduler;
pub mod tss;

use scheduler::SCHEDULER;

/// Initialize task management
pub fn init() {
    // Initialize TSS
    tss::init();
    
    // Initialize scheduler with idle task
    SCHEDULER.lock().init();
    
    crate::serial_println!("[TASK] Scheduler initialized with idle task");
}

/// Spawn a new kernel task
pub fn spawn_kernel_task(name: &str, entry: fn() -> !) -> u32 {
    const KERNEL_STACK_SIZE: usize = 4096 * 4; // 16 KB
    
    // Allocate kernel stack
    let stack = unsafe {
        let layout = alloc::alloc::Layout::from_size_align(KERNEL_STACK_SIZE, 16).unwrap();
        let ptr = alloc::alloc::alloc(layout);
        if ptr.is_null() {
            panic!("Failed to allocate kernel stack");
        }
        ptr as u64 + KERNEL_STACK_SIZE as u64
    };
    
    SCHEDULER.lock().spawn(
        alloc::string::String::from(name),
        entry as u64,
        stack
    )
}