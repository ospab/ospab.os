// Shared data structures

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    OutOfMemory,
    InvalidAddress,
    PageFault,
    // Add more
}

pub struct KernelConfig {
    pub total_memory: usize,
    pub kernel_start: usize,
    pub kernel_end: usize,
}

impl KernelConfig {
    pub const fn new() -> Self {
        KernelConfig {
            total_memory: 4 * 1024 * 1024 * 1024, // 4GB
            kernel_start: 0x100000, // 1MB
            kernel_end: 0x200000,   // 2MB
        }
    }
}