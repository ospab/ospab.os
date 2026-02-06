//! DOOM refactored for ospabOS v0.1.5
//! Uses syscalls for memory allocation and graphics

use crate::drivers::framebuffer;

// Doom configuration
pub const DOOMGENERIC_RESX: usize = 320;
pub const DOOMGENERIC_RESY: usize = 200;
pub const FRAMEBUFFER_SIZE: usize = DOOMGENERIC_RESX * DOOMGENERIC_RESY * 4; // RGBA

/// DOOM context with dynamic framebuffer
pub struct DoomContext {
    /// Framebuffer allocated via sys_malloc
    framebuffer_addr: u64,
    /// Keyboard state
    keys: DoomKeys,
    /// Running flag
    running: bool,
}

/// Doom keyboard state
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

impl DoomContext {
    /// Create new DOOM context with allocated framebuffer
    pub fn new() -> Result<Self, &'static str> {
        // Allocate framebuffer via sys_malloc
        let fb_addr = unsafe {
            syscall_malloc(FRAMEBUFFER_SIZE)
        };
        
        if fb_addr == 0 || fb_addr == !0 {
            return Err("Failed to allocate DOOM framebuffer");
        }
        
        // Zero out framebuffer
        unsafe {
            core::ptr::write_bytes(
                fb_addr as *mut u8,
                0,
                FRAMEBUFFER_SIZE,
            );
        }
        
        Ok(DoomContext {
            framebuffer_addr: fb_addr,
            keys: DoomKeys::new(),
            running: true,
        })
    }
    
    /// Get framebuffer pointer
    pub fn framebuffer(&self) -> *mut u32 {
        self.framebuffer_addr as *mut u32
    }
    
    /// Set pixel in DOOM framebuffer
    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < DOOMGENERIC_RESX && y < DOOMGENERIC_RESY {
            unsafe {
                let fb = self.framebuffer();
                *fb.add(y * DOOMGENERIC_RESX + x) = color;
            }
        }
    }
    
    /// Draw frame to screen
    pub fn draw_frame(&self) {
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
            let doom_fb = self.framebuffer();
            
            for y in 0..DOOMGENERIC_RESY {
                for x in 0..DOOMGENERIC_RESX {
                    let pixel = *doom_fb.add(y * DOOMGENERIC_RESX + x);
                    
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
    
    /// Update keyboard state
    pub fn update_keys(&mut self) {
        // Use syscall to read keyboard input (non-blocking)
        let mut buf = [0u8; 8];
        let n = unsafe { syscall_read(buf.as_mut_ptr(), buf.len()) };
        for i in 0..n {
            let c = buf[i] as char;
            match c {
                'w' | 'W' => self.keys.up = true,
                's' | 'S' => self.keys.down = true,
                'a' | 'A' => self.keys.left = true,
                'd' | 'D' => self.keys.right = true,
                ' ' => self.keys.use_key = true,
                '\x1b' => self.keys.escape = true,
                '\x03' => self.keys.escape = true, // Ctrl+C
                'q' | 'Q' => self.keys.escape = true,
                _ => {}
            }
        }
    }
    
    /// Check if DOOM should exit
    pub fn should_exit(&self) -> bool {
        self.keys.escape || !self.running
    }
    
    /// Stop DOOM
    pub fn stop(&mut self) {
        self.running = false;
    }
}

impl Drop for DoomContext {
    fn drop(&mut self) {
        // TODO: sys_free to deallocate framebuffer
        // For now, just leak it (will be freed when task terminates)
    }
}

/// Syscall wrapper for malloc
unsafe fn syscall_malloc(size: usize) -> u64 {
    let result: u64;
    core::arch::asm!(
        "mov rax, 6",        // SyscallNumber::Malloc
        "mov rdi, {size}",
        "syscall",
        size = in(reg) size,
        lateout("rax") result,
        options(nostack)
    );
    result
}

/// DOOM task entry point (v0.1.5)
pub fn doom_task_entry() -> ! {
    // Create DOOM context
    let mut doom = match DoomContext::new() {
        Ok(d) => d,
        Err(_e) => {
            // Write error via syscall
            let msg = "DOOM: Failed to initialize\n";
            unsafe {
                syscall_write(msg.as_ptr(), msg.len());
            }
            unsafe { syscall_exit(1) }
        }
    };
    
    // Write startup message
    let msg = "DOOM: Started successfully\n";
    unsafe {
        syscall_write(msg.as_ptr(), msg.len());
    }
    
    // Main DOOM loop
    loop {
        // Update game state
        doom.update_keys();
        
        // Render frame
        doom.draw_frame();
        
        // Check for exit
        if doom.should_exit() {
            break;
        }
        
        // Yield to scheduler
        unsafe { syscall_yield(); }
    }
    
    // Exit gracefully
    let msg = "DOOM: Exiting\n";
    unsafe {
        syscall_write(msg.as_ptr(), msg.len());
        syscall_exit(0);
    }
}

/// Syscall wrappers
unsafe fn syscall_write(buf: *const u8, len: usize) {
    core::arch::asm!(
        "mov rax, 2",
        "mov rdi, 1",
        "mov rsi, {buf}",
        "mov rdx, {len}",
        "syscall",
        buf = in(reg) buf,
        len = in(reg) len,
        lateout("rax") _,
        options(nostack)
    );
}

/// Syscall wrapper for yield
unsafe fn syscall_yield() {
    core::arch::asm!(
        "mov rax, 0",
        "syscall",
        lateout("rax") _,
        options(nostack)
    );
}

/// Syscall wrapper for exit
unsafe fn syscall_exit(code: i32) -> ! {
    let code_u64 = code as u64;
    core::arch::asm!(
        "mov rax, 4",
        "mov rdi, {code}",
        "syscall",
        code = in(reg) code_u64,
        options(noreturn)
    );
}

/// Syscall wrapper for read (keyboard input)
unsafe fn syscall_read(ptr: *mut u8, len: usize) -> usize {
    // Syscall number for read: 2 (example, adjust if needed)
    let ret: usize;
    core::arch::asm!(
        "mov rax, 2", // syscall number: read
        "mov rdi, {ptr}",
        "mov rsi, {len}",
        "syscall",
        "mov {ret}, rax",
        ptr = in(reg) ptr,
        len = in(reg) len,
        ret = out(reg) ret,
        options(nostack, preserves_flags)
    );
    ret
}

/// Initialize DOOM system
pub fn init() {
    framebuffer::print("DOOM system initialized (v0.1.5)\n");
}

/// Spawn DOOM as a task
pub fn spawn_doom_task() -> Result<u32, &'static str> {
    use crate::task::scheduler::SCHEDULER;
    use alloc::string::ToString;
    
    // Allocate stack for DOOM task
    const STACK_SIZE: usize = 64 * 1024; // 64 KB
    let stack = unsafe {
        syscall_malloc(STACK_SIZE)
    };
    
    if stack == 0 || stack == !0 {
        return Err("Failed to allocate DOOM stack");
    }
    
    // Spawn DOOM with its own address space
    let mut scheduler = SCHEDULER.lock();
    scheduler.spawn_with_address_space(
        "doom".to_string(),
        doom_task_entry as *const () as u64,
        stack + STACK_SIZE as u64,
    )
}
