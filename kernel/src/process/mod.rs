//! Process Management (inspired by Linux task_struct and scheduler)

pub mod task;
pub mod scheduler;

use alloc::vec::Vec;
use spin::Mutex;

static TASKS: Mutex<Vec<task::Task>> = Mutex::new(Vec::new());

pub fn init() {
    let init_task = task::Task::new(0, "init");
    TASKS.lock().push(init_task);
}

pub fn create_task(name: &str) -> Option<u32> {
    let mut tasks = TASKS.lock();
    let pid = tasks.len() as u32;
    let task = task::Task::new(pid, name);
    tasks.push(task);
    Some(pid)
}

pub fn get_current_pid() -> u32 {
    scheduler::get_current_pid()
}
