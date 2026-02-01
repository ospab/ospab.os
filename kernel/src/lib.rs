#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(naked_functions)]
#![feature(unsafe_attributes)]

extern crate alloc;

pub mod arch;
pub mod drivers;
pub mod common;
pub mod mem;
pub mod task;
pub mod sync;
pub mod interrupt;
pub mod gdt;
pub mod interrupts;
pub mod shell;
pub mod boot;
pub mod mm;
pub mod process;