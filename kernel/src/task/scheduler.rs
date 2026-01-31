use super::pcb::{ProcessControlBlock, TaskState};
use core::ptr;

pub struct Scheduler {
    current_task: *mut ProcessControlBlock,
    task_list: *mut ProcessControlBlock,
}

impl Scheduler {
    pub const fn new() -> Self {
        Scheduler {
            current_task: ptr::null_mut(),
            task_list: ptr::null_mut(),
        }
    }

    pub fn add_task(&mut self, task: *mut ProcessControlBlock) {
        unsafe {
            if self.task_list.is_null() {
                self.task_list = task;
                (*task).next = task; // Circular
            } else {
                let last = (*self.task_list).next;
                (*last).next = task;
                (*task).next = self.task_list;
            }
        }
    }

    pub fn schedule(&mut self) {
        if self.current_task.is_null() {
            self.current_task = self.task_list;
        } else {
            unsafe {
                (*self.current_task).save_context();
                self.current_task = (*self.current_task).next;
                (*self.current_task).restore_context();
            }
        }
    }

    pub fn yield_task(&mut self) {
        unsafe {
            if !self.current_task.is_null() {
                (*self.current_task).state = TaskState::Ready;
            }
        }
        self.schedule();
    }

    pub fn block_task(&mut self) {
        unsafe {
            if !self.current_task.is_null() {
                (*self.current_task).state = TaskState::Blocked;
            }
        }
        self.schedule();
    }

    pub fn unblock_task(&mut self, task: *mut ProcessControlBlock) {
        unsafe {
            (*task).state = TaskState::Ready;
        }
    }
}