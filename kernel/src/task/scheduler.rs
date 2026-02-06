//! Preemptive Round-Robin Scheduler for ospabOS v0.1.0

use super::pcb::{ProcessControlBlock, TaskState};
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::string::String;
use spin::Mutex;

/// Global scheduler instance
pub static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

pub struct Scheduler {
    /// Currently running task
    current: Option<Box<ProcessControlBlock>>,
    
    /// Ready queue (FIFO for round-robin)
    ready_queue: VecDeque<Box<ProcessControlBlock>>,
    
    /// Next PID to assign
    next_pid: u32,
    
    /// Total number of tasks
    task_count: usize,
}

impl Scheduler {
    pub const fn new() -> Self {
        Scheduler {
            current: None,
            ready_queue: VecDeque::new(),
            next_pid: 1,
            task_count: 0,
        }
    }
    
    /// Initialize scheduler with idle task
    pub fn init(&mut self) {
        let idle = ProcessControlBlock::new_idle();
        self.current = Some(idle);
        self.task_count = 1;
    }
    
    /// Spawn a new task
    pub fn spawn(&mut self, name: String, entry: u64, stack: u64) -> u32 {
        let pid = self.next_pid;
        self.next_pid += 1;
        
        let task = ProcessControlBlock::new(pid, name, entry, stack);
        self.ready_queue.push_back(task);
        self.task_count += 1;
        
        pid
    }
    
    /// Schedule next task (called from timer interrupt)
    pub fn schedule(&mut self) {
        // If no current task, just pick from queue
        if self.current.is_none() {
            if let Some(next) = self.ready_queue.pop_front() {
                self.current = Some(next);
            }
            return;
        }
        
        // Move current to back of queue if it's still ready
        let current = self.current.take().unwrap();
        
        let should_requeue = match current.state {
            TaskState::Running => {
                // Still running, requeue
                true
            },
            TaskState::Terminated => {
                // Drop the task
                self.task_count -= 1;
                false
            },
            TaskState::Blocked => {
                // TODO: Keep in separate blocked list
                true
            },
            _ => true,
        };
        
        if should_requeue {
            // Save current task
            let mut saved_current = current;
            saved_current.state = TaskState::Ready;
            self.ready_queue.push_back(saved_current);
        }
        
        // Pick next task from queue
        if let Some(mut next) = self.ready_queue.pop_front() {
            next.state = TaskState::Running;
            
            // Switch to task's address space if available
            if let Some(ref addr_space) = next.address_space {
                unsafe {
                    addr_space.switch_to();
                }
            }
            
            self.current = Some(next);
            
            // Note: Context switch would happen here in real implementation
            // For now, we just update the scheduler state
        }
    }
    
    /// Yield CPU voluntarily
    pub fn yield_task(&mut self) {
        if let Some(current) = &mut self.current {
            current.state = TaskState::Ready;
        }
        self.schedule();
    }
    
    /// Block current task
    pub fn block_current(&mut self) {
        if let Some(current) = &mut self.current {
            current.state = TaskState::Blocked;
        }
        self.schedule();
    }
    
    /// Terminate current task
    pub fn terminate_current(&mut self) {
        if let Some(current) = &mut self.current {
            current.state = TaskState::Terminated;
        }
        self.schedule();
    }
    
    /// Get current PID
    pub fn current_pid(&self) -> u32 {
        self.current.as_ref().map(|t| t.pid).unwrap_or(0)
    }
    
    /// Get task count
    pub fn task_count(&self) -> usize {
        self.task_count
    }
    
    /// Get mutable reference to current task
    pub fn current_task_mut(&mut self) -> Option<&mut ProcessControlBlock> {
        self.current.as_deref_mut()
    }
    
    /// Spawn a task with its own address space
    pub fn spawn_with_address_space(
        &mut self,
        name: String,
        entry: u64,
        stack: u64,
    ) -> Result<u32, &'static str> {
        use crate::mem::vmm::VMM;
        
        let pid = self.next_pid;
        self.next_pid += 1;
        
        // Create address space for the task
        let mut vmm = VMM.lock();
        let vmm = vmm.as_mut().ok_or("VMM not initialized")?;
        let addr_space = vmm.create_user_address_space()?;
        
        // Create task
        let mut task = ProcessControlBlock::new(pid, name, entry, stack);
        task.address_space = Some(addr_space);
        task.page_table = task.address_space.as_ref().unwrap().cr3.as_u64();
        
        self.ready_queue.push_back(task);
        self.task_count += 1;
        
        Ok(pid)
    }
}

/// Called from timer interrupt to trigger scheduling
pub fn timer_tick() {
    SCHEDULER.lock().schedule();
}