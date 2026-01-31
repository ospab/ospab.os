use super::spinlock::Spinlock;
use core::ptr;

pub struct Mutex {
    lock: Spinlock,
    owner: *mut u8, // Task ID or something
    wait_queue: *mut WaitNode,
}

#[repr(C)]
struct WaitNode {
    task: *mut u8,
    next: *mut WaitNode,
}

impl Mutex {
    pub const fn new() -> Self {
        Mutex {
            lock: Spinlock::new(),
            owner: ptr::null_mut(),
            wait_queue: ptr::null_mut(),
        }
    }

    pub fn lock(&mut self, current_task: *mut u8) {
        self.lock.lock();
        if !self.owner.is_null() && self.owner != current_task {
            // Add to wait queue
            // Allocate node using physical memory
            use crate::mem::physical::PhysicalAllocator;
            if let Some(addr) = PhysicalAllocator::allocate_page() {
                let node_ptr = addr as *mut WaitNode;
                unsafe {
                    ptr::write(node_ptr, WaitNode { task: current_task, next: self.wait_queue });
                }
                self.wait_queue = node_ptr;
                // Block task - simplified
            }
        } else {
            self.owner = current_task;
        }
        self.lock.unlock();
    }

    pub fn unlock(&mut self) {
        self.lock.lock();
        self.owner = ptr::null_mut();
        // Wake next in queue
        if !self.wait_queue.is_null() {
            unsafe {
                let _next_task = (*self.wait_queue).task;
                let next_node = (*self.wait_queue).next;
                // Free the node
                use crate::mem::physical::PhysicalAllocator;
                PhysicalAllocator::free_page(self.wait_queue as usize);
                self.wait_queue = next_node;
                // Unblock task - simplified
            }
        }
        self.lock.unlock();
    }
}