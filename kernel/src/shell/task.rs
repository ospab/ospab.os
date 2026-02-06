//! Shell Task - Runs shell as a background task in v0.1.0

use crate::drivers::{framebuffer, keyboard};
use alloc::format;

/// Shell task entry point
pub fn shell_task() -> ! {
    framebuffer::print("Shell task started\n");
    
    loop {
        // Wait for keyboard input
        if let Some(key) = keyboard::try_read_key() {
            // Process key through shell
            // This is simplified - real implementation would use proper IPC
            framebuffer::print(&format!("{}", key));
        }
        
        // Yield to other tasks
        // crate::syscall::dispatch_syscall(0, 0, 0, 0, 0); // sys_yield - TODO: Enable when ready
        
        // Small delay to avoid busy waiting
        for _ in 0..1000 {
            core::hint::spin_loop();
        }
    }
}

/// Initialize shell as background task
pub fn spawn_shell_task() -> u32 {
    crate::task::spawn_kernel_task("shell", shell_task)
}
