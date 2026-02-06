//! Kernel Heap Allocator with Limine Memory Map support
//!
//! This allocator scans the Limine Memory Map for USABLE regions
//! and initializes the heap dynamically based on available memory.

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use spin::Mutex;
use crate::boot::limine;

pub struct SimpleAllocator {
    heap_start: Mutex<Option<usize>>,
    heap_size: Mutex<usize>,
    allocated: Mutex<usize>,
}

unsafe impl GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Simple bump allocator
        let size = layout.size();
        let align = layout.align();
        
        if let Some(start) = *self.heap_start.lock() {
            let current = *self.allocated.lock();
            let aligned = (current + align - 1) & !(align - 1);
            
            if aligned + size <= *self.heap_size.lock() {
                *self.allocated.lock() = aligned + size;
                return (start + aligned) as *mut u8;
            }
        }
        
        null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // TODO: implement proper deallocation
    }
}

#[global_allocator]
static ALLOCATOR: SimpleAllocator = SimpleAllocator {
    heap_start: Mutex::new(None),
    heap_size: Mutex::new(0),
    allocated: Mutex::new(0),
};

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("Allocation error: {:?}", layout);
}

/// Initialize heap from Limine Memory Map
pub fn init() {
    // Find largest USABLE memory region from Limine
    let mut best_base: Option<u64> = None;
    let mut best_size: u64 = 0;
    
    if let Some(memmap) = limine::memory_map() {
        for entry in memmap {
            // Only use USABLE memory
            if entry.typ == limine::MEMMAP_USABLE {
                // Skip regions below 1MB (reserved for legacy hardware)
                if entry.base < 0x100000 {
                    continue;
                }
                
                // Find largest suitable region (at least 16MB)
                if entry.length >= 16 * 1024 * 1024 && entry.length > best_size {
                    best_base = Some(entry.base);
                    best_size = entry.length;
                }
            }
        }
    }
    
    if let Some(base) = best_base {
        // Use HHDM offset to access physical memory
        let hhdm = limine::hhdm_offset().unwrap_or(0);
        let heap_virt = (base + hhdm) as usize;
        
        // Cap heap size at 32MB for safety
        let heap_size = core::cmp::min(best_size as usize, 32 * 1024 * 1024);
        
        *ALLOCATOR.heap_start.lock() = Some(heap_virt);
        *ALLOCATOR.heap_size.lock() = heap_size;
        *ALLOCATOR.allocated.lock() = 0;
    } else {
        panic!("No suitable USABLE memory region found for heap!");
    }
}

/// Get heap statistics
pub fn heap_stats() -> (usize, usize, usize) {
    let start = ALLOCATOR.heap_start.lock().unwrap_or(0);
    let size = *ALLOCATOR.heap_size.lock();
    let allocated = *ALLOCATOR.allocated.lock();
    (start, size, allocated)
}
