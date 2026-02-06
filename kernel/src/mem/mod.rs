pub mod physical;
pub mod virt;
pub mod heap;
pub mod vmm;

pub fn init() {
    physical::PhysicalAllocator::init();
    // Heap init would be called with actual addresses
}

/// Initialize VMM after physical memory is ready
pub fn init_vmm() -> Result<(), &'static str> {
    vmm::init()
}