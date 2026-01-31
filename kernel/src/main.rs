#![no_std]
#![no_main]

extern crate ospab_os;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // Print to VGA or something
    ospab_os::drivers::VgaDriver::clear_screen();
    ospab_os::drivers::VgaDriver::print_str(0, 0, "Kernel Panic!", 0x04);
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Early serial output (use low-level polled serial for reliability)
    ospab_os::arch::x86_64::serial_init();
    ospab_os::arch::x86_64::serial_write_str("Kernel started\n");
    ospab_os::arch::x86_64::debug_port_write_str("Dstart\n");

    // Initialize kernel
    ospab_os::arch::init();
    ospab_os::arch::x86_64::serial_write_str("arch init done\n");
    ospab_os::arch::x86_64::debug_port_write_str("Darch done\n");
    ospab_os::mem::init();
    ospab_os::interrupt::init();
    ospab_os::task::init();

    // low-level serial
    ospab_os::arch::x86_64::serial_write_str("TomatoOS booted!\n");

    // Clear screen
    ospab_os::drivers::VgaDriver::clear_screen();
    ospab_os::drivers::VgaDriver::print_str(0, 0, "TomatoOS booted!", 0x0A);

    loop {
        ospab_os::arch::x86_64::halt();
    }
}