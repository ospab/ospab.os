use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
use core::cell::UnsafeCell;

struct HeapBlock {
    size: usize,
    next: *mut HeapBlock,
}

impl HeapBlock {
    fn new(size: usize) -> *mut Self {
        unsafe {
            // Allocate from physical
            use super::physical;
            if let Some(addr) = physical::allocate_page() {
                ptr::write(addr as *mut Self, HeapBlock { size, next: ptr::null_mut() });
                addr as *mut Self
            } else {
                ptr::null_mut()
            }
        }
    }
}

pub struct HeapAllocator {
    head: UnsafeCell<*mut HeapBlock>,
}

impl HeapAllocator {
    pub const fn new() -> Self {
        HeapAllocator {
            head: UnsafeCell::new(ptr::null_mut()),
        }
    }

    pub unsafe fn init(&self, _start: usize, size: usize) {
        *self.head.get() = HeapBlock::new(size);
    }

    fn find_free_block(&self, size: usize) -> *mut HeapBlock {
        let mut current = unsafe { *self.head.get() };
        while !current.is_null() {
            unsafe {
                if (*current).size >= size {
                    return current;
                }
                current = (*current).next;
            }
        }
        ptr::null_mut()
    }

    fn split_block(&self, block: *mut HeapBlock, size: usize) {
        unsafe {
            let remaining = (*block).size - size;
            if remaining > core::mem::size_of::<HeapBlock>() {
                let new_block = (block as usize + size) as *mut HeapBlock;
                ptr::write(new_block, HeapBlock { size: remaining, next: (*block).next });
                (*block).next = new_block;
                (*block).size = size;
            }
        }
    }

    fn merge_blocks(&self) {
        let mut current = unsafe { *self.head.get() };
        while !current.is_null() {
            unsafe {
                let next = (*current).next;
                if !next.is_null() && (current as usize + (*current).size) == next as usize {
                    (*current).size += (*next).size;
                    (*current).next = (*next).next;
                } else {
                    current = next;
                }
            }
        }
    }
}

unsafe impl GlobalAlloc for HeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(layout.align());
        let block = self.find_free_block(size);
        if !block.is_null() {
            self.split_block(block, size);
            (block as usize + core::mem::size_of::<HeapBlock>()) as *mut u8
        } else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let _block = (ptr as usize - core::mem::size_of::<HeapBlock>()) as *mut HeapBlock;
        self.merge_blocks();
    }
}