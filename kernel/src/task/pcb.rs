use core::ptr;
use core::arch::asm;

#[derive(Debug, Clone, Copy)]
pub enum TaskState {
    Running,
    Ready,
    Blocked,
    Terminated,
}

#[repr(C)]
pub struct ProcessControlBlock {
    pub id: u32,
    pub state: TaskState,
    pub priority: u8,
    pub stack_pointer: *mut u8,
    pub instruction_pointer: *mut u8,
    pub registers: [u64; 16], // rax, rbx, ..., r15
    pub next: *mut ProcessControlBlock,
}

impl ProcessControlBlock {
    pub fn new(id: u32, entry_point: *mut u8, stack_top: *mut u8) -> Self {
        ProcessControlBlock {
            id,
            state: TaskState::Ready,
            priority: 0,
            stack_pointer: stack_top,
            instruction_pointer: entry_point,
            registers: [0; 16],
            next: ptr::null_mut(),
        }
    }

    pub fn save_context(&mut self) {
        // Save registers using inline assembly
        unsafe {
            asm!(
                "mov [{}], rax",
                in(reg) &mut self.registers[0] as *mut u64,
            );
            // Simplified, save all registers
            // For full, need to save rsp, rip, etc.
        }
    }

    pub fn restore_context(&self) {
        // Restore registers
        unsafe {
            asm!(
                "mov rax, [{}]",
                in(reg) &self.registers[0] as *const u64,
            );
            // Simplified
        }
    }
}