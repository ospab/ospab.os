//! Terminal Service - Bridge between existing I/O and IPC layer
//! Wraps stable framebuffer and keyboard code without modifying it

use crate::drivers::{framebuffer, keyboard};
use crate::ipc::message::UIRequest;

/// Terminal service that uses existing stable I/O functions
pub struct TerminalService;

impl TerminalService {
    /// Create new terminal service
    pub const fn new() -> Self {
        Self
    }

    /// Process UI request using existing I/O functions
    pub fn process(&self, request: UIRequest) {
        match request {
            UIRequest::Print(text) => {
                // Use existing stable framebuffer print
                framebuffer::print(&text);
            }
            UIRequest::PrintLn(text) => {
                // Use existing stable framebuffer print with newline
                framebuffer::print(&text);
                framebuffer::print_char('\n');
            }
            UIRequest::Clear => {
                // Use existing clear function
                framebuffer::clear();
            }
            UIRequest::SetCursor { x, y } => {
                // Future: implement cursor positioning
                let _ = (x, y);
            }
            UIRequest::ReadLine => {
                // Keyboard input is handled by existing keyboard::process_scancodes()
                // in main loop, so this is a no-op for now
            }
        }
    }

    /// Poll and process keyboard events (uses existing stable code)
    pub fn poll_keyboard(&self) {
        // Use existing stable keyboard processing
        keyboard::process_scancodes();
    }
}

/// Global terminal service instance
static TERMINAL: spin::Mutex<Option<TerminalService>> = spin::Mutex::new(None);

/// Initialize terminal service
pub fn init() {
    let mut term = TERMINAL.lock();
    *term = Some(TerminalService::new());
}

/// Print text using terminal service
pub fn print(text: &str) {
    if let Some(ref term) = *TERMINAL.lock() {
        term.process(UIRequest::Print(text.into()));
    }
}

/// Print line using terminal service
pub fn println(text: &str) {
    if let Some(ref term) = *TERMINAL.lock() {
        term.process(UIRequest::PrintLn(text.into()));
    }
}

/// Clear screen using terminal service
pub fn clear() {
    if let Some(ref term) = *TERMINAL.lock() {
        term.process(UIRequest::Clear);
    }
}

/// Process keyboard input using terminal service
pub fn poll_input() {
    if let Some(ref term) = *TERMINAL.lock() {
        term.poll_keyboard();
    }
}
