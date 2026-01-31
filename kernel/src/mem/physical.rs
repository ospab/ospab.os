#![allow(unused_unsafe)]

const PAGE_SIZE: usize = 4096;
const TOTAL_MEMORY: usize = 4 * 1024 * 1024 * 1024; // 4GB
const TOTAL_PAGES: usize = TOTAL_MEMORY / PAGE_SIZE;
const BITMAP_SIZE: usize = TOTAL_PAGES / 8;

static mut BITMAP: [u8; BITMAP_SIZE] = [0; BITMAP_SIZE];
static mut INITIALIZED: bool = false;

pub struct PhysicalAllocator;

impl PhysicalAllocator {
    #[allow(unused_unsafe)]
    pub fn init() {
        unsafe {
            if !INITIALIZED {
                // Mark first 1MB as used (for kernel, etc.)
                let reserved_pages = (1024 * 1024) / PAGE_SIZE;
                for i in 0..reserved_pages {
                    Self::mark_used(i);
                }
                INITIALIZED = true;
            }
        }
    }

    pub fn allocate_page() -> Option<usize> {
        unsafe {
            for i in 0..BITMAP_SIZE {
                let byte = BITMAP[i];
                if byte != 0xFF {
                    for bit in 0..8 {
                        if (byte & (1 << bit)) == 0 {
                            let page_index = i * 8 + bit;
                            Self::mark_used(page_index);
                            return Some(page_index * PAGE_SIZE);
                        }
                    }
                }
            }
            None
        }
    }

    pub fn free_page(addr: usize) {
        let page_index = addr / PAGE_SIZE;
        unsafe {
            Self::mark_free(page_index);
        }
    }

    fn mark_used(page_index: usize) {
        unsafe {
            let byte_index = page_index / 8;
            let bit_index = page_index % 8;
            BITMAP[byte_index] |= 1 << bit_index;
        }
    }

    fn mark_free(page_index: usize) {
        unsafe {
            let byte_index = page_index / 8;
            let bit_index = page_index % 8;
            BITMAP[byte_index] &= !(1 << bit_index);
        }
    }

    pub fn is_used(page_index: usize) -> bool {
        unsafe {
            let byte_index = page_index / 8;
            let bit_index = page_index % 8;
            (BITMAP[byte_index] & (1 << bit_index)) != 0
        }
    }
}