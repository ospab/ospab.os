//! Message Bus - Central dispatcher for microkernel IPC

use alloc::collections::VecDeque;
use spin::Mutex;
use super::message::*;

/// Message queue for a service
struct ServiceQueue {
    messages: VecDeque<Message>,
}

impl ServiceQueue {
    const fn new() -> Self {
        Self {
            messages: VecDeque::new(),
        }
    }
}

/// Central message bus
pub struct MessageBus {
    vfs_queue: Mutex<ServiceQueue>,
    ui_queue: Mutex<ServiceQueue>,
    pkg_queue: Mutex<ServiceQueue>,
    system_queue: Mutex<ServiceQueue>,
}

impl MessageBus {
    /// Create new message bus
    pub const fn new() -> Self {
        Self {
            vfs_queue: Mutex::new(ServiceQueue::new()),
            ui_queue: Mutex::new(ServiceQueue::new()),
            pkg_queue: Mutex::new(ServiceQueue::new()),
            system_queue: Mutex::new(ServiceQueue::new()),
        }
    }

    /// Dispatch message to appropriate service queue
    pub fn dispatch(&self, msg: Message) {
        match msg {
            Message::FS(ref _req) => {
                let mut queue = self.vfs_queue.lock();
                queue.messages.push_back(msg);
            }
            Message::UI(ref _req) => {
                let mut queue = self.ui_queue.lock();
                queue.messages.push_back(msg);
            }
            Message::Pkg(ref _req) => {
                let mut queue = self.pkg_queue.lock();
                queue.messages.push_back(msg);
            }
            Message::System(ref _req) => {
                let mut queue = self.system_queue.lock();
                queue.messages.push_back(msg);
            }
        }
    }

    /// Get next message from VFS queue
    pub fn poll_vfs(&self) -> Option<Message> {
        let mut queue = self.vfs_queue.lock();
        queue.messages.pop_front()
    }

    /// Get next message from UI queue
    pub fn poll_ui(&self) -> Option<Message> {
        let mut queue = self.ui_queue.lock();
        queue.messages.pop_front()
    }

    /// Get next message from Package manager queue
    pub fn poll_pkg(&self) -> Option<Message> {
        let mut queue = self.pkg_queue.lock();
        queue.messages.pop_front()
    }

    /// Get next message from System queue
    pub fn poll_system(&self) -> Option<Message> {
        let mut queue = self.system_queue.lock();
        queue.messages.pop_front()
    }
}

/// Global message bus instance
static BUS: Mutex<Option<MessageBus>> = Mutex::new(None);

/// Initialize message bus
pub fn init() {
    let mut bus = BUS.lock();
    *bus = Some(MessageBus::new());
}

/// Send message to bus
pub fn send(msg: Message) {
    if let Some(ref bus) = *BUS.lock() {
        bus.dispatch(msg);
    }
}

/// Get message bus reference
pub fn get() -> Option<&'static MessageBus> {
    unsafe {
        if let Some(ref bus) = *BUS.lock() {
            Some(&*(bus as *const MessageBus))
        } else {
            None
        }
    }
}
