//! Memory Management Subsystem (inspired by Linux mm/)

pub mod heap_allocator;

/// Initialize memory management subsystem
pub fn init() {
    heap_allocator::init();
}
