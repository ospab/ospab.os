//! Simple Round-Robin Scheduler (inspired by Linux CFS)

use core::sync::atomic::{AtomicU32, Ordering};

static CURRENT_PID: AtomicU32 = AtomicU32::new(0);

pub fn schedule() {
    // Simple round-robin for now
    // TODO: Implement proper priority-based scheduling
}

pub fn get_current_pid() -> u32 {
    CURRENT_PID.load(Ordering::Relaxed)
}

pub fn set_current_pid(pid: u32) {
    CURRENT_PID.store(pid, Ordering::Relaxed);
}
