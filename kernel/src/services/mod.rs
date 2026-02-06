//! Services module - Microkernel services

pub mod terminal;
pub mod vfs;

pub use terminal::TerminalService;
pub use vfs::VFSService;
