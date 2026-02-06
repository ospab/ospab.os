// Doom port for ospabOS
// Based on doomgeneric - portable Doom implementation

pub mod task; // v0.1.0: DOOM as background task
pub mod v015; // v0.1.5: DOOM with syscalls and VMM

use crate::drivers::framebuffer;
use crate::drivers::timer;
use crate::drivers::keyboard;
use core::sync::atomic::{AtomicBool, Ordering};
use crate::services::vfs;
use crate::ipc::message::{FSRequest, FSResponse};
use alloc::string::String;

static DOOM_RUNNING: AtomicBool = AtomicBool::new(false);

// Doom configuration
pub const DOOMGENERIC_RESX: usize = 320;
pub const DOOMGENERIC_RESY: usize = 200;

// Doom framebuffer (320x200x4 bytes RGBA)
static mut DOOM_FRAMEBUFFER: [u32; DOOMGENERIC_RESX * DOOMGENERIC_RESY] = 
    [0; DOOMGENERIC_RESX * DOOMGENERIC_RESY];

// Doom keyboard state
pub struct DoomKeys {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub fire: bool,     // Ctrl
    pub use_key: bool,  // Space
    pub strafe: bool,   // Alt
    pub escape: bool,
}

impl DoomKeys {
    pub const fn new() -> Self {
        Self {
            up: false,
            down: false,
            left: false,
            right: false,
            fire: false,
            use_key: false,
            strafe: false,
            escape: false,
        }
    }
}

static mut DOOM_KEYS: DoomKeys = DoomKeys::new();

/// Initialize Doom framebuffer
pub fn init() {
    framebuffer::print("Initializing DOOM...\n");
    // Log to /var/log/doom.log
    doom_log("DOOM: init\n");
    unsafe {
        DOOM_FRAMEBUFFER = [0; DOOMGENERIC_RESX * DOOMGENERIC_RESY];
    }
}

/// Append message to /var/log/doom.log (best-effort)
fn doom_log(msg: &str) {
    let path = String::from("/var/log/doom.log");
    match vfs::process_request(FSRequest::ReadFile { path: path.clone() }) {
        FSResponse::FileData(mut data) => {
            data.extend_from_slice(msg.as_bytes());
            let _ = vfs::process_request(FSRequest::WriteFile { path: path.clone(), data });
        }
        FSResponse::Error(_) => {
            let _ = vfs::process_request(FSRequest::WriteFile { path: path.clone(), data: msg.as_bytes().to_vec() });
        }
        _ => {}
    }
}

/// Draw Doom frame to screen
pub fn draw_frame() {
    let fb_info = framebuffer::get_info();
    let fb_width = fb_info.width;
    let fb_height = fb_info.height;
    
    // Scale Doom 320x200 to screen resolution
    let scale_x = fb_width / DOOMGENERIC_RESX;
    let scale_y = fb_height / DOOMGENERIC_RESY;
    let scale = core::cmp::min(scale_x, scale_y).max(1);
    
    let offset_x = (fb_width - DOOMGENERIC_RESX * scale) / 2;
    let offset_y = (fb_height - DOOMGENERIC_RESY * scale) / 2;
    
    unsafe {
        for y in 0..DOOMGENERIC_RESY {
            for x in 0..DOOMGENERIC_RESX {
                let pixel = DOOM_FRAMEBUFFER[y * DOOMGENERIC_RESX + x];
                
                // Draw scaled pixel
                for dy in 0..scale {
                    for dx in 0..scale {
                        let screen_x = offset_x + x * scale + dx;
                        let screen_y = offset_y + y * scale + dy;
                        framebuffer::set_pixel(screen_x, screen_y, pixel);
                    }
                }
            }
        }
    }
}

/// Set pixel in Doom framebuffer
pub fn set_pixel(x: usize, y: usize, color: u32) {
    if x < DOOMGENERIC_RESX && y < DOOMGENERIC_RESY {
        unsafe {
            DOOM_FRAMEBUFFER[y * DOOMGENERIC_RESX + x] = color;
        }
    }
}

/// Get Doom framebuffer pointer
pub fn get_framebuffer() -> *mut u32 {
    core::ptr::addr_of_mut!(DOOM_FRAMEBUFFER).cast::<u32>()
}

/// Process keyboard input for Doom
pub fn process_input() {
    // Check multiple times to catch keys
    for _ in 0..10 {
        if let Some(key) = keyboard::try_read_key() {
            unsafe {
                match key {
                    'w' | 'W' => DOOM_KEYS.up = true,
                    's' | 'S' => DOOM_KEYS.down = true,
                    'a' | 'A' => DOOM_KEYS.left = true,
                    'd' | 'D' => DOOM_KEYS.right = true,
                    ' ' => DOOM_KEYS.use_key = true,
                    'q' | 'Q' => DOOM_KEYS.escape = true, // Q to quit
                    '\x1b' => DOOM_KEYS.escape = true,    // ESC
                    '\x03' => DOOM_KEYS.escape = true,    // Ctrl+C
                    '\x11' => DOOM_KEYS.escape = true,    // Ctrl+Q
                    _ => {}
                }
            }
        }
    }
}

/// Clear keyboard state (call after frame)
pub fn clear_input() {
    unsafe {
        DOOM_KEYS.up = false;
        DOOM_KEYS.down = false;
        DOOM_KEYS.left = false;
        DOOM_KEYS.right = false;
        DOOM_KEYS.fire = false;
        DOOM_KEYS.use_key = false;
        DOOM_KEYS.strafe = false;
        DOOM_KEYS.escape = false;
    }
}

/// Get current keyboard state
pub fn get_keys() -> &'static DoomKeys {
    unsafe { &*core::ptr::addr_of!(DOOM_KEYS) }
}

/// Check if Doom should quit
pub fn should_quit() -> bool {
    unsafe { DOOM_KEYS.escape }
}

/// Run Doom demo mode
pub fn run_demo() {
    framebuffer::clear_screen();
    framebuffer::print("=== DOOM for ospabOS ===\n\n");
    
    // Loading animation with status bar
    show_loading_screen();
    
    // Initialize Doom
    init();
    
    framebuffer::clear_screen();
    framebuffer::print("=== DOOM DEMO ===\n");
    framebuffer::print("Controls: W/A/S/D - Move | Space - Action | Q - Exit\n\n");
    framebuffer::print("Press Q to exit anytime...\n\n");
    sleep_ticks(50);
    
    DOOM_RUNNING.store(true, Ordering::Relaxed);
    doom_log("DOOM: demo start\n");
    
    // Demo animation (fire effect)
    let mut frame = 0u32;
    loop {
        // Check for exit (Q key or ESC)
        process_input();
        if should_quit() {
            doom_log("DOOM: exit requested\n");
            break;
        }
        
        // Draw fire effect demo
        draw_fire_effect(frame);
        draw_frame();
        draw_status_bar(frame);
        
        clear_input();
        frame = frame.wrapping_add(1);
        
        sleep_ticks(1);
    }
    
    DOOM_RUNNING.store(false, Ordering::Relaxed);
    framebuffer::clear_screen();
    framebuffer::print("DOOM exited. Thanks for playing!\n");
    doom_log("DOOM: exited\n");
}

/// Show loading screen with progress bar
fn show_loading_screen() {
    framebuffer::print("Loading DOOM...\n\n");
    
    let steps = 20;
    for i in 0..=steps {
        // Draw progress bar
        framebuffer::print("\r[");
        for j in 0..steps {
            if j < i {
                framebuffer::print("#");
            } else {
                framebuffer::print("-");
            }
        }
        framebuffer::print("] ");
        
        // Print percentage
        let percent = (i * 100) / steps;
        print_number(percent);
        framebuffer::print("%");
        
        sleep_ticks(2);
    }
    
    framebuffer::print("\n\nDOOM loaded successfully!\n");
    
    sleep_ticks(20);
}

fn sleep_ticks(ticks: u64) {
    let start = timer::get_jiffies();
    while timer::get_jiffies() < start + ticks {
        x86_64::instructions::hlt();
    }
}

/// Draw status bar at bottom of screen
fn draw_status_bar(frame: u32) {
    let fb_info = framebuffer::get_info();
    let status_y = fb_info.height - 20;
    
    // Draw black background for status bar
    for y in status_y..fb_info.height {
        for x in 0..fb_info.width {
            framebuffer::set_pixel(x, y, 0x000000);
        }
    }
    
    // Draw status text (simplified - just frame counter and exit hint)
    let status_text_y = status_y + 6;
    let _fps = (frame % 60) as u8; // For future FPS display
    
    // Draw "FPS: XX" and "Press Q to exit"
    draw_status_text(10, status_text_y, b"DOOM DEMO");
    draw_status_text(fb_info.width / 2 - 50, status_text_y, b"Press Q to EXIT");
}

/// Draw simple text on screen (for status bar)
fn draw_status_text(x: usize, y: usize, text: &[u8]) {
    for (i, &ch) in text.iter().enumerate() {
        let char_x = x + i * 8;
        if ch != b' ' {
            // Draw 8x8 character
            for dy in 0..8 {
                for dx in 0..8 {
                    framebuffer::set_pixel(char_x + dx, y + dy, 0xFFFFFF);
                }
            }
        }
    }
}

/// Print a number to framebuffer
fn print_number(n: u64) {
    if n == 0 {
        framebuffer::print("0");
        return;
    }
    
    let mut buf = [0u8; 20];
    let mut i = 0;
    let mut num = n;
    
    while num > 0 {
        buf[i] = b'0' + (num % 10) as u8;
        num /= 10;
        i += 1;
    }
    
    for j in (0..i).rev() {
        framebuffer::print_char(buf[j] as char);
    }
}

/// Draw fire effect (demo until full Doom is integrated)
fn draw_fire_effect(frame: u32) {
    // Simple gradient animation instead of float math
    for y in 0..DOOMGENERIC_RESY {
        for x in 0..DOOMGENERIC_RESX {
            // Integer-based pattern
            let fx = (x * 256 / DOOMGENERIC_RESX) as u32;
            let fy = (y * 256 / DOOMGENERIC_RESY) as u32;
            let t = frame & 0xFF;
            
            // XOR pattern with time offset
            let val = ((fx + t) ^ (fy + t)) & 0xFF;
            
            // Red-orange fire palette
            let intensity = val as u8;
            let red = intensity;
            let green = intensity / 2;
            let blue = if intensity > 200 { intensity - 200 } else { 0 };
            
            let color = ((red as u32) << 16) | ((green as u32) << 8) | (blue as u32);
            set_pixel(x, y, color);
        }
    }
    
    // Draw "DOOM" text in center
    let text = "D O O M";
    let text_x = DOOMGENERIC_RESX / 2 - text.len() * 4;
    let text_y = DOOMGENERIC_RESY / 2;
    
    // Simple pixel text
    for (i, ch) in text.chars().enumerate() {
        if ch != ' ' {
            draw_char_pixel(text_x + i * 8, text_y, 0xFFFFFF);
        }
    }
}

/// Draw a character as a simple block
fn draw_char_pixel(x: usize, y: usize, color: u32) {
    for dy in 0..8 {
        for dx in 0..8 {
            if x + dx < DOOMGENERIC_RESX && y + dy < DOOMGENERIC_RESY {
                set_pixel(x + dx, y + dy, color);
            }
        }
    }
}

/// Check if Doom is running
pub fn is_running() -> bool {
    DOOM_RUNNING.load(Ordering::Relaxed)
}
