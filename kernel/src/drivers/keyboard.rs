//! Keyboard driver for ospabOS
//! Production-ready: uses atomic ring buffer, no static mut in ISR path

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;
use x86_64::instructions::port::Port;
use crate::drivers::framebuffer;

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

use core::sync::atomic::AtomicU8;

// ============================================================================
// KEYBOARD STATE (protected by Mutex, accessed only from main loop)
// ============================================================================

struct KeyboardState {
    keyboard: Option<Keyboard<layouts::Us104Key, ScancodeSet1>>,
    cmd_buf: [u8; CMD_BUFFER_SIZE],
    cmd_len: usize,
    history: [[u8; CMD_BUFFER_SIZE]; HISTORY_SIZE],
    history_lens: [usize; HISTORY_SIZE],
    history_count: usize,
    history_pos: Option<usize>, // Current position in history (None = not navigating)
}

static STATE: Mutex<KeyboardState> = Mutex::new(KeyboardState {
    keyboard: None,
    cmd_buf: [0u8; CMD_BUFFER_SIZE],
    cmd_len: 0,
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
            handle_char(character);
        }
        DecodedKey::RawKey(key) => {
            // Handle arrow keys for history navigation
            use pc_keyboard::KeyCode;
            match key {
                KeyCode::ArrowUp => handle_arrow_up(),
                KeyCode::ArrowDown => handle_arrow_down(),
                _ => {}
            }
        }
    }
}

fn handle_char(c: char) {
    let mut state = STATE.lock();
    
    match c {
        '\n' | '\r' => {
            // Reset history navigation
            state.history_pos = None;
            
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
            framebuffer::print("[ospab]~> ");
        }
        '\x08' => {
            // Backspace - exit history mode
            if state.cmd_len > 0 {
                state.history_pos = None;
                state.cmd_len -= 1;
                drop(state);
                framebuffer::print_char('\x08');
            }
        }
        c if c.is_ascii() && !c.is_control() => {
            // Exit history mode on typing
            state.history_pos = None;
            if state.cmd_len < CMD_BUFFER_SIZE - 1 {
                let idx = state.cmd_len;
                state.cmd_buf[idx] = c as u8;
                state.cmd_len += 1;
                drop(state);
                framebuffer::print_char(c);
            }
        }
        _ => {}
    }
}

fn handle_arrow_up() {
    let mut state = STATE.lock();
    
    if state.history_count == 0 {
        drop(state);
        return;
    }
    
    let next_pos = match state.history_pos {
        None => 0,
        Some(pos) if pos + 1 < state.history_count => pos + 1,
        Some(_) => {
            drop(state);
            return; // Already at oldest
        }
    };
    
    // Clear current line
    let cmd_len = state.cmd_len;
    for _ in 0..cmd_len {
        framebuffer::print_char('\x08');
    }
    
    // Load history entry
    state.history_pos = Some(next_pos);
    state.cmd_len = state.history_lens[next_pos];
    state.cmd_buf = state.history[next_pos];
    
    // Copy to local buffer before dropping lock
    let display_len = state.cmd_len;
    let mut display_buf = [0u8; CMD_BUFFER_SIZE];
    display_buf[..display_len].copy_from_slice(&state.cmd_buf[..display_len]);
    drop(state);
    
    // Display
    if let Ok(s) = core::str::from_utf8(&display_buf[..display_len]) {
        framebuffer::print(s);
    }
}

fn handle_arrow_down() {
    let mut state = STATE.lock();
    
    let next_pos = match state.history_pos {
        None => {
            drop(state);
            return;
        }
        Some(0) => {
            // Go to empty line - clear current
            let cmd_len = state.cmd_len;
            for _ in 0..cmd_len {
                framebuffer::print_char('\x08');
            }
            state.history_pos = None;
            state.cmd_len = 0;
            drop(state);
            return;
        }
        Some(pos) => pos - 1,
    };
    
    // Clear current line
    let cmd_len = state.cmd_len;
    for _ in 0..cmd_len {
        framebuffer::print_char('\x08');
    }
    
    // Load history entry
    state.history_pos = Some(next_pos);
    state.cmd_len = state.history_lens[next_pos];
    state.cmd_buf = state.history[next_pos];
    
    // Copy to local buffer before dropping lock
    let display_len = state.cmd_len;
    let mut display_buf = [0u8; CMD_BUFFER_SIZE];
    display_buf[..display_len].copy_from_slice(&state.cmd_buf[..display_len]);
    drop(state);
    
    // Display
    if let Ok(s) = core::str::from_utf8(&display_buf[..display_len]) {
        framebuffer::print(s);
    }
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
    
    match cmd {
        "" => {}
        "help" => {
            framebuffer::print("Available commands:\n");
            framebuffer::print("  help    - Show this help\n");
            framebuffer::print("  history - Show command history\n");
            framebuffer::print("  clear   - Clear screen\n");
            framebuffer::print("  status  - Show system status\n");
            framebuffer::print("  about   - About ospabOS\n");
            framebuffer::print("  reboot  - Reboot system\n");
            framebuffer::print("Use UP/DOWN arrows to navigate history\n");
        }
        "history" => {
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
        "clear" => {
            // Skip clear - it's slow
            framebuffer::print("(clear disabled - slow)\n");
        }
        "status" => {
            framebuffer::print("System Status: Running\n");
            framebuffer::print("Kernel: ospabOS v0.1.0\n");
            framebuffer::print("Arch: x86_64\n");
        }
        "about" => {
            framebuffer::print("\n");
            framebuffer::print("  ospabOS - A hobby operating system\n");
            framebuffer::print("  Written in Rust, booted via Limine\n");
            framebuffer::print("  Version 0.1.0\n");
            framebuffer::print("\n");
        }
        "reboot" => {
            framebuffer::print("Rebooting...\n");
            unsafe {
                let mut port = Port::<u8>::new(0x64);
                port.write(0xFE);
            }
        }
        _ => {
            framebuffer::print("Error: unknown command '");
            framebuffer::print(cmd);
            framebuffer::print("'\n");
            framebuffer::print("Type 'help' for available commands.\n");
        }
    }
}