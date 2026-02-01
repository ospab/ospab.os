//! Task structure (inspired by Linux task_struct)

use alloc::string::String;

#[derive(Clone)]
pub struct Task {
    pub pid: u32,
    pub name: String,
    pub state: TaskState,
    pub priority: u8,
    pub time_slice: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Running,
    Ready,
    Blocked,
    Zombie,
}

impl Task {
    pub fn new(pid: u32, name: &str) -> Self {
        Self {
            pid,
            name: String::from(name),
            state: TaskState::Ready,
            priority: 100,
            time_slice: 10,
        }
    }
}
