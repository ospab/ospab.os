pub mod physical;
pub mod virt;
pub mod heap;

pub fn init() {
    physical::PhysicalAllocator::init();
    // Heap init would be called with actual addresses
}