#![no_std]
#![no_main]

extern crate ospab_os;

use core::panic::PanicInfo;

use ospab_os::drivers::vga_buffer::{self, Color};
use ospab_os::gdt;
use ospab_os::interrupts;
use ospab_os::drivers::keyboard::keyboard;
use ospab_os::shell::shell;
use ospab_os::println;
use x86_64::instructions::interrupts::{enable as interrupts_enable, disable as interrupts_disable};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga_buffer::init();
    vga_buffer::set_color(Color::LightRed, Color::Black);
    println!("KERNEL PANIC: {}", info);
    loop { ospab_os::arch::x86_64::halt(); }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    gdt::init();
    interrupts::init_idt();
    ospab_os::interrupts::pics().lock().initialize();
    interrupts_enable();

    vga_buffer::init();
    println!("ospabOS Kernel v.0.1.0 Ready");
    vga_buffer::print("> ");

    loop {
        // Process buffered input from keyboard and deliver to shell
        interrupts_disable();
        if let Some(mut kb) = keyboard().try_lock() {
            if let Some(mut sh) = shell().try_lock() {
                while let Some(c) = kb.get_char() {
                    sh.on_keypress(c);
                }
            }
        }
        interrupts_enable();
        ospab_os::arch::x86_64::halt();
    }
}