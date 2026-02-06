//! Inter-Process Communication (IPC) Layer for ospabOS
//! Message-passing microkernel architecture

pub mod message;
pub mod bus;

pub use message::Message;
pub use bus::MessageBus;
