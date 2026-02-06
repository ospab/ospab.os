//! Process Control Block for ospabOS v0.1.0

use alloc::boxed::Box;
use alloc::string::String;
use core::ptr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Running,
    Ready,
    Blocked,
    Terminated,
}

/// CPU context saved during task switch
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TaskContext {
    // Callee-saved registers (as per x86_64 calling convention)
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
    // Return address (rip)
    pub rip: u64,
}

impl TaskContext {
    pub const fn new() -> Self {
        TaskContext {
            r15: 0, r14: 0, r13: 0, r12: 0,
            rbx: 0, rbp: 0, rip: 0,
        }
    }
}

/// Process Control Block (Task descriptor)
pub struct ProcessControlBlock {
    pub pid: u32,
    pub state: TaskState,
    pub priority: u8,
    pub name: String,
    
    // Context switching
    pub context: TaskContext,
    pub kernel_stack: u64,
    pub user_stack: u64,
    
    // Memory management
    pub page_table: u64, // CR3 value
    pub address_space: Option<crate::mem::vmm::AddressSpace>, // VMM address space
    
    // Linked list for scheduler
    pub next: *mut ProcessControlBlock,
}

// SAFETY: PCB is only accessed from scheduler which is behind a Mutex
unsafe impl Send for ProcessControlBlock {}

impl ProcessControlBlock {
    /// Create a new task
    pub fn new(pid: u32, name: String, entry_point: u64, stack: u64) -> Box<Self> {
        let mut pcb = Box::new(ProcessControlBlock {
            pid,
            state: TaskState::Ready,
            priority: 0,
            name,
            context: TaskContext::new(),
            kernel_stack: stack,
            user_stack: 0,
            page_table: 0, // Use kernel page table for now
            address_space: None, // Will be set later
            next: ptr::null_mut(),
        });
        
        // Initialize context for first run
        pcb.context.rip = entry_point;
        pcb.context.rbp = stack;
        
        pcb
    }
    
    /// Create idle task (runs when no other task is ready)
    pub fn new_idle() -> Box<Self> {
        Self::new(0, String::from("idle"), idle_task as *const () as u64, 0)
    }
}

/// Idle task - just HLT in loop
fn idle_task() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

/// Switch from old task to new task
/// 
/// This is called from scheduler with interrupts disabled
/// 
/// # Safety
/// Must be called with valid task contexts
#[unsafe(naked)]
pub unsafe extern "C" fn switch_context(old: *mut TaskContext, new: *const TaskContext) {
    core::arch::naked_asm!(
        // Save old context (callee-saved registers)
        "mov [rdi + 0x00], r15",
        "mov [rdi + 0x08], r14",
        "mov [rdi + 0x10], r13",
        "mov [rdi + 0x18], r12",
        "mov [rdi + 0x20], rbx",
        "mov [rdi + 0x28], rbp",
        
        // Save return address (rip)
        "mov rax, [rsp]",
        "mov [rdi + 0x30], rax",
        
        // Restore new context
        "mov r15, [rsi + 0x00]",
        "mov r14, [rsi + 0x08]",
        "mov r13, [rsi + 0x10]",
        "mov r12, [rsi + 0x18]",
        "mov rbx, [rsi + 0x20]",
        "mov rbp, [rsi + 0x28]",
        
        // Jump to new task
        "mov rax, [rsi + 0x30]",
        "jmp rax"
    );
}