//! DOOM Task - Runs DOOM as a schedulable task in v0.1.0

/// DOOM task entry point
pub fn doom_task() -> ! {
    // Note: DOOM's actual run() function is called from shell command
    // This task would just wait for DOOM to be launched
    loop {
        x86_64::instructions::hlt();
    }
}

/// Spawn DOOM as background task
pub fn spawn_doom_task() -> u32 {
    crate::task::spawn_kernel_task("doom", doom_task)
}
