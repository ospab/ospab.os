//! Physical Frame Allocator for ospabOS v0.1.0
//! Uses bitmap-based allocation with proper locking

use spin::Mutex;
use alloc::format;

const PAGE_SIZE: usize = 4096;
const TOTAL_MEMORY: usize = 128 * 1024 * 1024; // 128 MB (realistic for now)
const TOTAL_FRAMES: usize = TOTAL_MEMORY / PAGE_SIZE;
const BITMAP_SIZE: usize = (TOTAL_FRAMES + 7) / 8; // Round up

/// Global frame allocator
pub static FRAME_ALLOCATOR: Mutex<FrameAllocator> = Mutex::new(FrameAllocator::new());

pub struct FrameAllocator {
    bitmap: [u8; BITMAP_SIZE],
    next_free: usize,
    total_frames: usize,
    used_frames: usize,
}

impl FrameAllocator {
    pub const fn new() -> Self {
        FrameAllocator {
            bitmap: [0; BITMAP_SIZE],
            next_free: 0,
            total_frames: TOTAL_FRAMES,
            used_frames: 0,
        }
    }
    
    /// Initialize allocator (mark kernel memory as used)
    pub fn init(&mut self, kernel_start: usize, kernel_end: usize) {
        // Mark first 1 MB as reserved (BIOS, VGA, etc.)
        let reserved_frames = (1024 * 1024) / PAGE_SIZE;
        for i in 0..reserved_frames {
            self.mark_used(i);
        }
        
        // Mark kernel memory as used
        let kernel_start_frame = kernel_start / PAGE_SIZE;
        let kernel_end_frame = (kernel_end + PAGE_SIZE - 1) / PAGE_SIZE;
        
        for i in kernel_start_frame..kernel_end_frame {
            self.mark_used(i);
        }
        
        crate::serial_println!("[MEM] Frame allocator initialized");
        crate::serial_println!("      Total frames: {}", self.total_frames);
        crate::serial_println!("      Used frames: {}", self.used_frames);
        crate::serial_println!("      Free frames: {}", self.total_frames - self.used_frames);
    }
    
    /// Allocate a physical frame
    pub fn allocate(&mut self) -> Option<usize> {
        // Start from last known free position
        for i in self.next_free..self.total_frames {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            
            if (self.bitmap[byte_idx] & (1 << bit_idx)) == 0 {
                // Found free frame
                self.bitmap[byte_idx] |= 1 << bit_idx;
                self.used_frames += 1;
                self.next_free = i + 1;
                return Some(i * PAGE_SIZE);
            }
        }
        
        // Wrap around and search from beginning
        for i in 0..self.next_free {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            
            if (self.bitmap[byte_idx] & (1 << bit_idx)) == 0 {
                self.bitmap[byte_idx] |= 1 << bit_idx;
                self.used_frames += 1;
                self.next_free = i + 1;
                return Some(i * PAGE_SIZE);
            }
        }
        
        None // Out of memory
    }
    
    /// Free a physical frame
    pub fn free(&mut self, addr: usize) {
        let frame = addr / PAGE_SIZE;
        
        if frame >= self.total_frames {
            return;
        }
        
        let byte_idx = frame / 8;
        let bit_idx = frame % 8;
        
        if (self.bitmap[byte_idx] & (1 << bit_idx)) != 0 {
            self.bitmap[byte_idx] &= !(1 << bit_idx);
            self.used_frames -= 1;
            
            if frame < self.next_free {
                self.next_free = frame;
            }
        }
    }
    
    /// Mark frame as used
    fn mark_used(&mut self, frame: usize) {
        if frame >= self.total_frames {
            return;
        }
        
        let byte_idx = frame / 8;
        let bit_idx = frame % 8;
        
        if (self.bitmap[byte_idx] & (1 << bit_idx)) == 0 {
            self.bitmap[byte_idx] |= 1 << bit_idx;
            self.used_frames += 1;
        }
    }
    
    /// Get memory statistics
    pub fn stats(&self) -> (usize, usize, usize) {
        (self.total_frames, self.used_frames, self.total_frames - self.used_frames)
    }
}

/// Get memory statistics (total, used, free frames)
pub fn stats() -> (usize, usize, usize) {
    let allocator = FRAME_ALLOCATOR.lock();
    allocator.stats()
}

/// Allocate a physical page
pub fn allocate_page() -> Option<usize> {
    FRAME_ALLOCATOR.lock().allocate()
}

/// Free a physical page
pub fn free_page(addr: usize) {
    FRAME_ALLOCATOR.lock().free(addr)
}

/// Initialize physical memory allocator
pub fn init() {
    // Physical allocator is initialized via FRAME_ALLOCATOR
}