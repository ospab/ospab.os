#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(c_variadic)]

extern crate alloc;

pub mod arch;
pub mod drivers;
pub mod common;
pub mod mem;
pub mod fs;
pub mod task;
pub mod sync;
pub mod interrupt;
pub mod gdt;
pub mod interrupts;
pub mod boot;
pub mod mm;
pub mod process;

// Microkernel IPC architecture
pub mod ipc;
pub mod services;
pub mod shell;
pub mod apps;
pub mod grape;  // Grape text editor
pub mod auth;     // User authentication system
pub mod net;      // Network stack
pub mod doom;   // DOOM port
pub mod power;  // Power management (shutdown/reboot)
pub mod loader; // Executable loaders

// v0.1.0 "Foundation" additions
pub mod syscall; // Syscall interface