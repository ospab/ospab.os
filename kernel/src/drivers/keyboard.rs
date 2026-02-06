//! Keyboard driver for ospabOS
//! Production-ready: uses atomic ring buffer, no static mut in ISR path

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;
use x86_64::instructions::port::Port;
use crate::drivers::framebuffer;
use crate::services::vfs;
use crate::ipc::message::FSRequest;
use alloc::string::String;

// PS/2 Controller ports (Intel 8042)
const KBD_DATA_PORT: u16 = 0x60;     // Data port
const KBD_STATUS_PORT: u16 = 0x64;   // Status register (read)
const KBD_COMMAND_PORT: u16 = 0x64;  // Command register (write)

// Status register bits
const STATUS_OUTPUT_FULL: u8 = 0x01;  // Output buffer full
const STATUS_INPUT_FULL: u8 = 0x02;   // Input buffer full

// Commands
const CMD_READ_CONFIG: u8 = 0x20;     // Read controller configuration byte
const CMD_WRITE_CONFIG: u8 = 0x60;    // Write controller configuration byte
const CMD_DISABLE_PORT1: u8 = 0xAD;   // Disable first PS/2 port
const CMD_ENABLE_PORT1: u8 = 0xAE;    // Enable first PS/2 port

// Configuration byte bits
const CONFIG_IRQ1_ENABLED: u8 = 0x01;    // Enable keyboard interrupt
const CONFIG_TRANSLATION: u8 = 0x40;     // Enable scancode translation

// Local serial print for debugging
fn serial_print(msg: &[u8]) {
    let mut port: Port<u8> = Port::new(0x3F8);
    for &b in msg {
        unsafe { port.write(b); }
    }
}

#[allow(dead_code)]
#[allow(dead_code)]
fn serial_print_hex(val: u8) {
    const HEX: &[u8] = b"0123456789ABCDEF";
    serial_print(&[HEX[(val >> 4) as usize], HEX[(val & 0xF) as usize]]);
}

const CMD_BUFFER_SIZE: usize = 256;
const SCANCODE_BUFFER_SIZE: usize = 128;
const HISTORY_SIZE: usize = 5;  // Reduced from 20 to 5

// ============================================================================
// LOCK-FREE SCANCODE RING BUFFER (for ISR)
// ============================================================================

/// Atomic scancode ring buffer - safe for interrupt handlers
static SCANCODE_BUF: [AtomicU8; SCANCODE_BUFFER_SIZE] = {
    const INIT: AtomicU8 = AtomicU8::new(0);
    [INIT; SCANCODE_BUFFER_SIZE]
};
static SCANCODE_READ: AtomicUsize = AtomicUsize::new(0);
static SCANCODE_WRITE: AtomicUsize = AtomicUsize::new(0);
static INITIALIZED: AtomicBool = AtomicBool::new(false);

// Track Ctrl state and extended prefix for scancode handling
static CTRL_PRESSED: AtomicBool = AtomicBool::new(false);
static EXTENDED_FLAG: AtomicBool = AtomicBool::new(false);

use core::sync::atomic::AtomicU8;

// ============================================================================
// KEYBOARD STATE (protected by Mutex, accessed only from main loop)
// ============================================================================

struct KeyboardState {
    keyboard: Option<Keyboard<layouts::Us104Key, ScancodeSet1>>,
    cmd_buf: [u8; CMD_BUFFER_SIZE],
    cmd_len: usize,
    cursor_pos: usize, // Cursor position in cmd_buf (0..=cmd_len)
    history: [[u8; CMD_BUFFER_SIZE]; HISTORY_SIZE],
    history_lens: [usize; HISTORY_SIZE],
    history_count: usize,
    history_pos: Option<usize>, // Current position in history (None = not navigating)
}

static STATE: Mutex<KeyboardState> = Mutex::new(KeyboardState {
    keyboard: None,
    cmd_buf: [0u8; CMD_BUFFER_SIZE],
    cmd_len: 0,
    cursor_pos: 0,
    history: [[0u8; CMD_BUFFER_SIZE]; HISTORY_SIZE],
    history_lens: [0usize; HISTORY_SIZE],
    history_count: 0,
    history_pos: None,
});

/// Wait for input buffer to be empty (safe to send command)
fn wait_input_ready() -> bool {
    let mut status_port: Port<u8> = Port::new(KBD_STATUS_PORT);
    for _ in 0..10000 {
        unsafe {
            if (status_port.read() & STATUS_INPUT_FULL) == 0 {
                return true;
            }
        }
        // Small delay
        for _ in 0..10 { core::hint::spin_loop(); }
    }
    false
}

/// Wait for output buffer to have data
fn wait_output_ready() -> bool {
    let mut status_port: Port<u8> = Port::new(KBD_STATUS_PORT);
    for _ in 0..10000 {
        unsafe {
            if (status_port.read() & STATUS_OUTPUT_FULL) != 0 {
                return true;
            }
        }
        for _ in 0..10 { core::hint::spin_loop(); }
    }
    false
}

/// Initialize PS/2 controller (based on Linux i8042 driver)
/// Does NOT enable hardware interrupts - call enable_hw_irq() later
pub fn init() {
    serial_print(b"[KBD] Initializing PS/2 controller...\r\n");
    
    let mut cmd_port: Port<u8> = Port::new(KBD_COMMAND_PORT);
    let mut data_port: Port<u8> = Port::new(KBD_DATA_PORT);
    
    unsafe {
        // Step 1: Disable PS/2 port to prevent interference
        if !wait_input_ready() {
            serial_print(b"[KBD] Timeout waiting for input buffer\r\n");
        }
        cmd_port.write(CMD_DISABLE_PORT1);
        
        // Step 2: Flush output buffer
        for _ in 0..16 {
            let mut status_port: Port<u8> = Port::new(KBD_STATUS_PORT);
            if (status_port.read() & STATUS_OUTPUT_FULL) != 0 {
                data_port.read(); // Discard
            } else {
                break;
            }
        }
        
        // Step 3: Read current configuration
        if !wait_input_ready() { return; }
        cmd_port.write(CMD_READ_CONFIG);
        
        if !wait_output_ready() {
            serial_print(b"[KBD] Failed to read config\r\n");
            return;
        }
        let mut config = data_port.read();
        
        // Step 4: Set desired configuration:
        // - DO NOT enable interrupts yet (will do it later)
        // - Enable translation to Set 1 (bit 6)
        config &= !CONFIG_IRQ1_ENABLED; // Explicitly disable IRQ
        config |= CONFIG_TRANSLATION; // Enable scancode translation
        
        if !wait_input_ready() { return; }
        cmd_port.write(CMD_WRITE_CONFIG);
        
        if !wait_input_ready() { return; }
        data_port.write(config);
        
        // Step 5: Enable PS/2 port
        if !wait_input_ready() { return; }
        cmd_port.write(CMD_ENABLE_PORT1);
        
        // Step 6: Flush any pending data after enable
        for _ in 0..16 {
            let mut status_port: Port<u8> = Port::new(KBD_STATUS_PORT);
            if (status_port.read() & STATUS_OUTPUT_FULL) != 0 {
                data_port.read(); // Discard
            } else {
                break;
            }
        }
    }
    
    // Step 7: Initialize decoder (under lock)
    {
        let mut state = STATE.lock();
        state.keyboard = Some(Keyboard::new(
            layouts::Us104Key,
            ScancodeSet1,
            HandleControl::Ignore  // Back to Ignore for normal operation
        ));
    }
    
    // Mark as initialized (atomic)
    INITIALIZED.store(true, Ordering::Release);
    
    serial_print(b"[KBD] PS/2 controller initialized (IRQ disabled)\r\n");
}

/// Enable interrupts at hardware level (PS/2 controller)
/// Call this AFTER sti, at the very end of initialization
pub fn enable_hw_irq() {
    serial_print(b"[KBD] Enabling keyboard hardware IRQ...\r\n");
    
    // First unmask IRQ1 in PIC
    crate::interrupts::enable_irq(1);
    
    // Then enable at PS/2 controller level
    let mut cmd_port: Port<u8> = Port::new(KBD_COMMAND_PORT);
    let mut data_port: Port<u8> = Port::new(KBD_DATA_PORT);
    
    unsafe {
        // Read current config
        if !wait_input_ready() { return; }
        cmd_port.write(CMD_READ_CONFIG);
        
        if !wait_output_ready() { return; }
        let mut config = data_port.read();
        
        // Enable IRQ1
        config |= CONFIG_IRQ1_ENABLED;
        
        // Write back
        if !wait_input_ready() { return; }
        cmd_port.write(CMD_WRITE_CONFIG);
        
        if !wait_input_ready() { return; }
        data_port.write(config);
    }
    
    serial_print(b"[KBD] Keyboard IRQ enabled\r\n");
}

/// Legacy alias for enable_hw_irq
pub fn enable_interrupts_at_hw_level() {
    enable_hw_irq();
}

/// Called from ISR - queue scancode using atomic operations (lock-free)
pub fn queue_scancode(scancode: u8) {
    if !INITIALIZED.load(Ordering::Acquire) {
        return; // Not ready yet
    }
    
    let write = SCANCODE_WRITE.load(Ordering::Relaxed);
    let next_write = (write + 1) % SCANCODE_BUFFER_SIZE;
    let read = SCANCODE_READ.load(Ordering::Relaxed);
    
    if next_write != read {
        SCANCODE_BUF[write].store(scancode, Ordering::Relaxed);
        SCANCODE_WRITE.store(next_write, Ordering::Release);
    }
    // If buffer full, drop scancode
}

/// Called from main loop - process queued scancodes
pub fn process_scancodes() {
    if !INITIALIZED.load(Ordering::Acquire) {
        return;
    }
    
    // Process up to 32 scancodes per call
    let mut count = 0;
    loop {
        let read = SCANCODE_READ.load(Ordering::Acquire);
        let write = SCANCODE_WRITE.load(Ordering::Acquire);
        
        if read == write || count >= 32 {
            break;
        }
        
        let scancode = SCANCODE_BUF[read].load(Ordering::Relaxed);
        SCANCODE_READ.store((read + 1) % SCANCODE_BUFFER_SIZE, Ordering::Release);
        
        handle_scancode(scancode);
        count += 1;
    }
}

pub fn handle_scancode(scancode: u8) {
    // Track extended prefix and Ctrl make/release based on raw scancodes
    if scancode == 0xE0 {
        EXTENDED_FLAG.store(true, Ordering::Relaxed);
    } else {
        let _extended = EXTENDED_FLAG.load(Ordering::Relaxed);
        match scancode {
            0x1D => CTRL_PRESSED.store(true, Ordering::Relaxed),  // Ctrl press (left/right)
            0x9D => CTRL_PRESSED.store(false, Ordering::Relaxed), // Ctrl release (left)
            _ => {}
        }
        // reset extended flag after processing a non-0xE0 byte
        EXTENDED_FLAG.store(false, Ordering::Relaxed);
    }

    let mut state = STATE.lock();
    
    let kb = match state.keyboard.as_mut() {
        Some(k) => k,
        None => return,
    };
    
    let key_event = match kb.add_byte(scancode) {
        Ok(Some(ev)) => ev,
        _ => return,
    };
    
    let key = match kb.process_keyevent(key_event) {
        Some(k) => k,
        None => return,
    };
    
    // Drop state lock before calling framebuffer (prevents potential deadlock)
    drop(state);
    
    match key {
        DecodedKey::Unicode(character) => {
            // If Ctrl is held and a letter is pressed, map to control character (e.g., Ctrl+C -> '\x03')
            if CTRL_PRESSED.load(Ordering::Relaxed) && character.is_ascii_alphabetic() {
                let ctl = ((character.to_ascii_lowercase() as u8) - b'a' + 1) as u8;
                handle_char(ctl as char);
            } else {
                handle_char(character);
            }
        }
        DecodedKey::RawKey(key) => {
            // Handle arrow keys for history navigation and cursor movement
            use pc_keyboard::KeyCode;
            match key {
                KeyCode::ArrowUp => handle_arrow_up(),
                KeyCode::ArrowDown => handle_arrow_down(),
                KeyCode::ArrowLeft => handle_arrow_left(),
                KeyCode::ArrowRight => handle_arrow_right(),
                _ => {}
            }
        }
    }
}

fn handle_char(c: char) {
    let mut state = STATE.lock();
    framebuffer::hide_cursor();
    
    match c {
        '\x03' => {
            // Ctrl+C - cancel current input
            state.history_pos = None;
            state.cmd_len = 0;
            state.cursor_pos = 0;
            drop(state);

            framebuffer::print("^C\n");
            let prompt = crate::shell::get_prompt();
            framebuffer::print(&prompt);
        }
        '\n' | '\r' => {
            // Reset history navigation and cursor
            state.history_pos = None;
            state.cursor_pos = 0;
            
            framebuffer::print_char('\n');
            
            // Save to history if not empty
            if state.cmd_len > 0 {
                // Shift history and add new command
                if state.history_count < HISTORY_SIZE {
                    state.history_count += 1;
                }
                for i in (1..state.history_count).rev() {
                    state.history[i] = state.history[i - 1];
                    state.history_lens[i] = state.history_lens[i - 1];
                }
                state.history[0] = state.cmd_buf;
                state.history_lens[0] = state.cmd_len;
            }
            
            // Execute command needs the buffer
            let cmd_len = state.cmd_len;
            let cmd_buf: [u8; CMD_BUFFER_SIZE] = state.cmd_buf;
            state.cmd_len = 0;
            drop(state); // Drop lock before command execution
            
            execute_command_impl(&cmd_buf[..cmd_len]);
            
            // Show prompt with current directory
            let prompt = crate::shell::get_prompt();
            framebuffer::print(&prompt);
        }
        '\x08' => {
            // Backspace - delete char before cursor
            state.history_pos = None;
            
            if state.cursor_pos > 0 {
                let cursor_pos = state.cursor_pos;
                let cmd_len = state.cmd_len;
                
                // Shift characters left from cursor position
                for i in cursor_pos..cmd_len {
                    state.cmd_buf[i - 1] = state.cmd_buf[i];
                }
                state.cmd_len -= 1;
                state.cursor_pos -= 1;
                
                // Redraw line from cursor to end
                let chars_to_redraw = state.cmd_len - state.cursor_pos;
                let mut redraw_buf = [0u8; CMD_BUFFER_SIZE];
                redraw_buf[..chars_to_redraw].copy_from_slice(&state.cmd_buf[state.cursor_pos..state.cmd_len]);
                
                drop(state);
                
                // Move cursor back, redraw rest of line, clear last char, move cursor back
                framebuffer::print_char('\x08');
                if chars_to_redraw > 0 {
                    if let Ok(s) = core::str::from_utf8(&redraw_buf[..chars_to_redraw]) {
                        framebuffer::print(s);
                    }
                }
                framebuffer::print_char(' '); // Clear last char
                // Move cursor back to position
                for _ in 0..=chars_to_redraw {
                    framebuffer::print_char('\x08');
                }
            }
        }
        // Tab completion
        '\t' => {
            // Find current token before cursor
            let cursor_pos = state.cursor_pos;
            let mut start = cursor_pos;
            while start > 0 && state.cmd_buf[start - 1] != b' ' {
                start -= 1;
            }
            let prefix = core::str::from_utf8(&state.cmd_buf[start..cursor_pos]).unwrap_or("");

            // Collect candidates from /bin and current directory
            let mut candidates: alloc::vec::Vec<String> = alloc::vec::Vec::new();
            if let crate::ipc::message::FSResponse::DirListing(list) = vfs::process_request(FSRequest::ListDir { path: String::from("/bin") }) {
                for name in list { if name.starts_with(prefix) { candidates.push(name); } }
            }
            if let crate::ipc::message::FSResponse::DirListing(list) = vfs::process_request(FSRequest::ListDir { path: String::from(".") }) {
                for name in list { if name.starts_with(prefix) { candidates.push(name); } }
            }

            if candidates.len() == 1 {
                // Insert rest of the candidate
                let completion = &candidates[0];
                let suffix = &completion[prefix.len()..];
                // Insert suffix into buffer
                let s_bytes = suffix.as_bytes();
                let add_len = s_bytes.len();
                let insert_at = state.cursor_pos;
                let old_len = state.cmd_len;
                if old_len + add_len >= CMD_BUFFER_SIZE { drop(state); framebuffer::show_cursor(); return; }
                // Shift right (use locals to avoid re-borrowing `state` inside loop bounds)
                for i in (insert_at..old_len).rev() {
                    state.cmd_buf[i + add_len] = state.cmd_buf[i];
                }
                for (i, &b) in s_bytes.iter().enumerate() {
                    state.cmd_buf[insert_at + i] = b;
                }
                state.cmd_len = old_len + add_len;
                state.cursor_pos = insert_at + add_len;

                // Redraw remainder of line
                let redraw_len = state.cmd_len - state.cursor_pos + 1;
                let start_pos = state.cursor_pos - 1;
                let mut redraw_buf = [0u8; CMD_BUFFER_SIZE];
                redraw_buf[..redraw_len].copy_from_slice(&state.cmd_buf[start_pos..state.cmd_len]);
                drop(state);
                if let Ok(s) = core::str::from_utf8(&redraw_buf[..redraw_len]) { framebuffer::print(s); }
                for _ in 1..redraw_len { framebuffer::print_char('\x08'); }
                framebuffer::show_cursor();
                return;
            } else if candidates.len() > 1 {
                // List candidates
                framebuffer::print_char('\n');
                for name in &candidates { framebuffer::print(name); framebuffer::print("  "); }
                framebuffer::print_char('\n');
                // Reprint prompt and buffer
                let prompt = crate::shell::get_prompt();
                framebuffer::print(&prompt);
                if let Ok(s) = core::str::from_utf8(&state.cmd_buf[..state.cmd_len]) { framebuffer::print(s); }
                framebuffer::show_cursor();
                return;
            } else {
                // No candidates - do nothing
            }
        }
        c if c.is_ascii() && !c.is_control() => {
            // Exit history mode on typing
            state.history_pos = None;
            
            if state.cmd_len < CMD_BUFFER_SIZE - 1 {
                let cursor_pos = state.cursor_pos;
                let cmd_len = state.cmd_len;
                
                // Insert character at cursor position
                if cursor_pos < cmd_len {
                    // Shift characters right from cursor
                    for i in (cursor_pos..cmd_len).rev() {
                        state.cmd_buf[i + 1] = state.cmd_buf[i];
                    }
                }
                
                state.cmd_buf[cursor_pos] = c as u8;
                state.cmd_len += 1;
                state.cursor_pos += 1;
                
                // Redraw from cursor to end
                let chars_to_redraw = state.cmd_len - state.cursor_pos + 1;
                let start_pos = state.cursor_pos - 1;
                let mut redraw_buf = [0u8; CMD_BUFFER_SIZE];
                redraw_buf[..chars_to_redraw].copy_from_slice(&state.cmd_buf[start_pos..state.cmd_len]);
                
                drop(state);
                
                if let Ok(s) = core::str::from_utf8(&redraw_buf[..chars_to_redraw]) {
                    framebuffer::print(s);
                }
                
                // Move cursor back to correct position
                for _ in 1..chars_to_redraw {
                    framebuffer::print_char('\x08');
                }
            }
        }
        _ => {}
    }
    framebuffer::show_cursor();
}

fn handle_arrow_up() {
    let mut state = STATE.lock();
    framebuffer::hide_cursor();
    
    if state.history_count == 0 {
        drop(state);
        framebuffer::show_cursor();
        return;
    }
    
    let next_pos = match state.history_pos {
        None => 0,
        Some(pos) if pos + 1 < state.history_count => pos + 1,
        Some(_) => {
            drop(state);
            framebuffer::show_cursor();
            return; // Already at oldest
        }
    };
    
    // Clear current line completely on screen
    let old_len = state.cmd_len;
    let old_cursor = state.cursor_pos;
    
    // Move cursor to end if not there
    for _ in old_cursor..old_len {
        framebuffer::print_char(' '); // Move forward
    }
    // Now backspace and erase everything
    for _ in 0..old_len {
        framebuffer::print_char('\x08'); // Move back
        framebuffer::print_char(' ');    // Erase
        framebuffer::print_char('\x08'); // Move back again
    }
    
    // Load history entry
    state.history_pos = Some(next_pos);
    state.cmd_len = state.history_lens[next_pos];
    state.cmd_buf = state.history[next_pos];
    state.cursor_pos = state.cmd_len; // Move cursor to end
    
    // Copy to local buffer before dropping lock
    let display_len = state.cmd_len;
    let mut display_buf = [0u8; CMD_BUFFER_SIZE];
    display_buf[..display_len].copy_from_slice(&state.cmd_buf[..display_len]);
    drop(state);
    
    // Display new command
    if let Ok(s) = core::str::from_utf8(&display_buf[..display_len]) {
        framebuffer::print(s);
    }
    framebuffer::show_cursor();
}

fn handle_arrow_left() {
    let mut state = STATE.lock();
    framebuffer::hide_cursor();
    
    if state.cursor_pos > 0 {
        state.cursor_pos -= 1;
        drop(state);
        // Move cursor left using backspace (without deleting)
        framebuffer::print_char('\x08');
        framebuffer::show_cursor();
        return;
    }
    drop(state);
    framebuffer::show_cursor();
}

fn handle_arrow_right() {
    let mut state = STATE.lock();
    framebuffer::hide_cursor();
    
    if state.cursor_pos < state.cmd_len {
        // Get character at cursor to redraw it (moves cursor forward)
        let c = state.cmd_buf[state.cursor_pos] as char;
        state.cursor_pos += 1;
        drop(state);
        framebuffer::print_char(c);
        framebuffer::show_cursor();
        return;
    }
    drop(state);
    framebuffer::show_cursor();
}

fn handle_arrow_down() {
    let mut state = STATE.lock();
    framebuffer::hide_cursor();
    
    let next_pos = match state.history_pos {
        None => {
            drop(state);
            framebuffer::show_cursor();
            return;
        }
        Some(0) => {
            // Go to empty line - clear current completely
            let old_len = state.cmd_len;
            let old_cursor = state.cursor_pos;
            
            // Move to end
            for _ in old_cursor..old_len {
                framebuffer::print_char(' ');
            }
            // Erase everything
            for _ in 0..old_len {
                framebuffer::print_char('\x08');
                framebuffer::print_char(' ');
                framebuffer::print_char('\x08');
            }
            
            state.history_pos = None;
            state.cmd_len = 0;
            state.cursor_pos = 0;
            drop(state);
            framebuffer::show_cursor();
            return;
        }
        Some(pos) => pos - 1,
    };
    
    // Clear current line completely on screen
    let old_len = state.cmd_len;
    let old_cursor = state.cursor_pos;
    
    // Move to end
    for _ in old_cursor..old_len {
        framebuffer::print_char(' ');
    }
    // Erase everything
    for _ in 0..old_len {
        framebuffer::print_char('\x08');
        framebuffer::print_char(' ');
        framebuffer::print_char('\x08');
    }
    
    // Load history entry
    state.history_pos = Some(next_pos);
    state.cmd_len = state.history_lens[next_pos];
    state.cmd_buf = state.history[next_pos];
    state.cursor_pos = state.cmd_len; // Move cursor to end
    
    // Copy to local buffer before dropping lock
    let display_len = state.cmd_len;
    let mut display_buf = [0u8; CMD_BUFFER_SIZE];
    display_buf[..display_len].copy_from_slice(&state.cmd_buf[..display_len]);
    drop(state);
    
    // Display new command
    if let Ok(s) = core::str::from_utf8(&display_buf[..display_len]) {
        framebuffer::print(s);
    }
    framebuffer::show_cursor();
}

#[allow(dead_code)]
fn clear_current_line(state: &mut spin::MutexGuard<KeyboardState>) {
    for _ in 0..state.cmd_len {
        framebuffer::print_char('\x08');
    }
}

fn execute_command_impl(cmd_bytes: &[u8]) {
    let cmd = match core::str::from_utf8(cmd_bytes) {
        Ok(s) => s.trim(),
        Err(_) => {
            framebuffer::print("Error: invalid UTF-8\n");
            return;
        }
    };
    
    // Delegate to shell module
    crate::shell::execute_command(cmd);
}

/// Print command history (called from shell)
pub fn print_history() {
    let state = STATE.lock();
    if state.history_count == 0 {
        framebuffer::print("No history yet\n");
    } else {
        framebuffer::print("Command history:\n");
        for i in 0..state.history_count {
            let cmd_bytes = &state.history[i][..state.history_lens[i]];
            if let Ok(s) = core::str::from_utf8(cmd_bytes) {
                framebuffer::print("  ");
                // Print index (newest first)
                let idx = state.history_count - i;
                if idx < 10 {
                    framebuffer::print_char(('0' as u8 + idx as u8) as char);
                } else {
                    framebuffer::print_char(('0' as u8 + (idx / 10) as u8) as char);
                    framebuffer::print_char(('0' as u8 + (idx % 10) as u8) as char);
                }
                framebuffer::print(". ");
                framebuffer::print(s);
                framebuffer::print_char('\n');
            }
        }
    }
}

/// Read a single key in blocking mode (for text editors)
/// Returns Some(key) when key is pressed, None should not happen
pub fn read_key_blocking() -> Option<char> {
    loop {
        // Dequeue scancode from atomic ring buffer
        let read = SCANCODE_READ.load(Ordering::Relaxed);
        let write = SCANCODE_WRITE.load(Ordering::Acquire);
        
        if read != write {
            let scancode = SCANCODE_BUF[read].load(Ordering::Relaxed);
            SCANCODE_READ.store((read + 1) % SCANCODE_BUFFER_SIZE, Ordering::Release);
            
            // Process scancode through keyboard decoder
            let mut state = STATE.lock();
            if let Some(ref mut kbd) = state.keyboard {
                if let Ok(Some(key_event)) = kbd.add_byte(scancode) {
                    if let Some(key) = kbd.process_keyevent(key_event) {
                        match key {
                            DecodedKey::Unicode(c) => return Some(c),
                            DecodedKey::RawKey(_) => {
                                // Ignore raw keys for now
                                continue;
                            }
                        }
                    }
                }
            }
        }
        
        // Yield CPU while waiting
        core::hint::spin_loop();
    }
}

/// Editor key type for text editors - includes both characters and navigation
#[derive(Debug, Clone, Copy)]
pub enum EditorKey {
    Char(char),
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    PageUp,
    PageDown,
    Home,
    End,
    Delete,
}

/// Read a key for text editor (handles navigation keys)
pub fn read_editor_key_blocking() -> Option<EditorKey> {
    loop {
        // Dequeue scancode from atomic ring buffer
        let read = SCANCODE_READ.load(Ordering::Relaxed);
        let write = SCANCODE_WRITE.load(Ordering::Acquire);
        
        if read != write {
            let scancode = SCANCODE_BUF[read].load(Ordering::Relaxed);
            SCANCODE_READ.store((read + 1) % SCANCODE_BUFFER_SIZE, Ordering::Release);
            
            // Process scancode through keyboard decoder
            let mut state = STATE.lock();
            if let Some(ref mut kbd) = state.keyboard {
                if let Ok(Some(key_event)) = kbd.add_byte(scancode) {
                    if let Some(key) = kbd.process_keyevent(key_event) {
                        match key {
                            DecodedKey::Unicode(c) => return Some(EditorKey::Char(c)),
                            DecodedKey::RawKey(raw) => {
                                use pc_keyboard::KeyCode;
                                match raw {
                                    KeyCode::ArrowUp => return Some(EditorKey::ArrowUp),
                                    KeyCode::ArrowDown => return Some(EditorKey::ArrowDown),
                                    KeyCode::ArrowLeft => return Some(EditorKey::ArrowLeft),
                                    KeyCode::ArrowRight => return Some(EditorKey::ArrowRight),
                                    KeyCode::PageUp => return Some(EditorKey::PageUp),
                                    KeyCode::PageDown => return Some(EditorKey::PageDown),
                                    KeyCode::Home => return Some(EditorKey::Home),
                                    KeyCode::End => return Some(EditorKey::End),
                                    KeyCode::Delete => return Some(EditorKey::Delete),
                                    _ => continue, // Ignore other raw keys
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Yield CPU while waiting
        core::hint::spin_loop();
    }
}

/// Try to read a key without blocking (for DOOM and games)
pub fn try_read_key() -> Option<char> {
    let read = SCANCODE_READ.load(Ordering::Relaxed);
    let write = SCANCODE_WRITE.load(Ordering::Acquire);
    
    if read != write {
        let scancode = SCANCODE_BUF[read].load(Ordering::Relaxed);
        SCANCODE_READ.store((read + 1) % SCANCODE_BUFFER_SIZE, Ordering::Release);
        
        // Process scancode
        let mut state = STATE.lock();
        if let Some(ref mut kbd) = state.keyboard {
            if let Ok(Some(key_event)) = kbd.add_byte(scancode) {
                if let Some(key) = kbd.process_keyevent(key_event) {
                    if let DecodedKey::Unicode(c) = key {
                        return Some(c);
                    }
                }
            }
        }
    }
    
    None
}