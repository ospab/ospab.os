//! Kernel Heap Allocator (similar to Linux kmalloc/kfree)

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use spin::Mutex;

const HEAP_SIZE: usize = 1024 * 1024; // 1 MB kernel heap

#[repr(align(4096))]
struct HeapSpace {
    data: [u8; HEAP_SIZE],
}

static mut HEAP_SPACE: HeapSpace = HeapSpace { data: [0; HEAP_SIZE] };

pub struct SimpleAllocator {
    heap_start: Mutex<Option<usize>>,
    heap_size: Mutex<usize>,
    allocated: Mutex<usize>,
}

unsafe impl GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Simple bump allocator for now
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

pub fn init() {
    unsafe {
        let heap_addr = core::ptr::addr_of!(HEAP_SPACE.data) as usize;
        *ALLOCATOR.heap_start.lock() = Some(heap_addr);
        *ALLOCATOR.heap_size.lock() = HEAP_SIZE;
        *ALLOCATOR.allocated.lock() = 0;
    }
    // Can't use serial_print here as it might allocate
}
