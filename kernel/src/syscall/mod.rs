//! Syscall Interface for ospabOS v0.1.0
//! Implements x86_64 syscall/sysret mechanism

use crate::task::scheduler::SCHEDULER;
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use core::sync::atomic::{AtomicBool, Ordering};

pub mod dispatcher;
pub mod abi;
pub mod entry;

/// Syscall numbers (stable ABI)
#[derive(Debug, Clone, Copy)]
#[repr(u64)]
pub enum SyscallNumber {
    Yield = 0,
    Spawn = 1,
    Write = 2,
    Read = 3,
    Exit = 4,
    GetPid = 5,
    Malloc = 6,  // New: dynamic memory allocation
    Open = 7,
    Exec = 8,
    DrawChar = 9,
    Chdir = 10,
    GetCwd = 11,
    ListDir = 12,
    Uptime = 13,
    Shutdown = 14,
    Reboot = 15,
}

static SPAWN_WORKER_STARTED: AtomicBool = AtomicBool::new(false);
static SPAWN_QUEUE: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Initialize syscall handling
pub fn init() {
    unsafe {
        // Enable syscall/sysret support
        enable_syscall_support();
    }
}

/// Enable syscall support in CPU
unsafe fn enable_syscall_support() {
    use x86_64::registers::model_specific::*;
    use crate::gdt;
    
    let selectors = gdt::selectors();
    let kernel_cs = selectors.kernel_code.0 as u64;
    let user_cs = (selectors.user_code.0 as u64) | 3;

    // Set STAR MSR (CS/SS for syscall/sysret)
    // STAR[47:32] = Kernel CS
    // STAR[63:48] = User CS
    Msr::new(0xC0000081).write((user_cs << 48) | (kernel_cs << 32));
    
    // Set LSTAR MSR (syscall entry point)
    Msr::new(0xC0000082).write(entry::syscall_handler as *const () as u64);
    
    // Set SFMASK MSR (RFLAGS mask)
    Msr::new(0xC0000084).write(0x200); // Clear IF
    
    // Enable SYSCALL in EFER
    let mut efer = Efer::read();
    efer |= EferFlags::SYSTEM_CALL_EXTENSIONS;
    Efer::write(efer);
}

/// Dispatch syscall from user space
pub fn dispatch_syscall(num: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> u64 {
    match num {
        0 => sys_yield(),
        1 => sys_spawn(arg1 as *const u8, arg2 as usize),
        2 => sys_write(arg1, arg2 as *const u8, arg3 as usize),
        3 => sys_read(arg1, arg2 as *mut u8, arg3 as usize),
        4 => sys_exit(arg1 as i32),
        5 => sys_getpid(),
        6 => sys_malloc(arg1 as usize), // New: memory allocation
        7 => sys_open(arg1 as *const u8, arg2),
        8 => sys_exec(arg1 as *const u8),
        9 => sys_draw_char(arg1, arg2, arg3, arg4, arg5),
        10 => sys_chdir(arg1 as *const u8),
        11 => sys_getcwd(arg1 as *mut u8, arg2 as usize),
        12 => sys_listdir(arg1 as *const u8, arg2 as *mut u8, arg3 as usize),
        13 => sys_uptime(),
        14 => sys_shutdown(),
        15 => sys_reboot(),
        _ => !0, // Invalid syscall
    }
}

/// Syscall implementations
fn sys_yield() -> u64 {
    SCHEDULER.lock().yield_task();
    0
}

fn sys_spawn(path_ptr: *const u8, _name_len: usize) -> u64 {
    let path = match read_c_string(path_ptr) {
        Some(p) => p,
        None => return !0,
    };

    SPAWN_QUEUE.lock().push(path);

    if !SPAWN_WORKER_STARTED.swap(true, Ordering::SeqCst) {
        return crate::task::spawn_kernel_task("spawn-worker", spawn_worker) as u64;
    }

    0
}

fn sys_write(fd: u64, buf: *const u8, len: usize) -> u64 {
    if buf.is_null() || len == 0 {
        return 0;
    }

    let mut scheduler = SCHEDULER.lock();
    let current = match scheduler.current_task_mut() {
        Some(task) => task,
        None => return !0,
    };

    let handle = match current.fd_table.get_mut(fd as u32) {
        Ok(h) => h,
        Err(_) => return !0,
    };

    unsafe {
        let slice = core::slice::from_raw_parts(buf, len);
        match handle.write(slice) {
            Ok(written) => written as u64,
            Err(_) => !0,
        }
    }
}

fn sys_read(fd: u64, buf: *mut u8, len: usize) -> u64 {
    if buf.is_null() || len == 0 {
        return 0;
    }

    let mut scheduler = SCHEDULER.lock();
    let current = match scheduler.current_task_mut() {
        Some(task) => task,
        None => return !0,
    };

    let handle = match current.fd_table.get_mut(fd as u32) {
        Ok(h) => h,
        Err(_) => return !0,
    };

    unsafe {
        let slice = core::slice::from_raw_parts_mut(buf, len);
        match handle.read(slice) {
            Ok(read) => read as u64,
            Err(_) => !0,
        }
    }
}

fn sys_exit(_code: i32) -> u64 {
    SCHEDULER.lock().terminate_current();
    0
}

fn sys_getpid() -> u64 {
    SCHEDULER.lock().current_pid() as u64
}

fn sys_malloc(size: usize) -> u64 {
    use crate::mem::vmm::VMM;
    use crate::task::scheduler::SCHEDULER;
    
    if size == 0 {
        return 0; // NULL pointer for zero allocation
    }
    
    // Get current task's address space
    let mut scheduler = SCHEDULER.lock();
    let current_task = match scheduler.current_task_mut() {
        Some(task) => task,
        None => return !0, // No current task
    };
    
    // Allocate memory in task's address space
    let mut vmm = VMM.lock();
    let vmm = match vmm.as_mut() {
        Some(v) => v,
        None => return !0, // VMM not initialized
    };
    
    if let Some(ref mut addr_space) = current_task.address_space {
        match vmm.allocate_user_memory(size, addr_space) {
            Ok(virt_addr) => virt_addr.as_u64(),
            Err(_) => !0, // Allocation failed
        }
    } else {
        !0 // No address space for task
    }
}

fn sys_open(path_ptr: *const u8, _flags: u64) -> u64 {
    let path = match read_c_string(path_ptr) {
        Some(p) => p,
        None => return !0,
    };

    let handle = match crate::services::vfs::open(&path, _flags) {
        Ok(h) => h,
        Err(_) => return !0,
    };

    let mut scheduler = SCHEDULER.lock();
    let current = match scheduler.current_task_mut() {
        Some(task) => task,
        None => return !0,
    };

    current.fd_table.insert(handle) as u64
}

fn sys_exec(path_ptr: *const u8) -> u64 {
    let path = match read_c_string(path_ptr) {
        Some(p) => p,
        None => return !0,
    };

    match exec_user_path(&path) {
        Ok(_) => 0,
        Err(_) => !0,
    }
}

fn sys_draw_char(x: u64, y: u64, ch: u64, fg: u64, bg: u64) -> u64 {
    let row = y as usize;
    let col = x as usize;
    let ch = (ch as u8) as char;
    let fg = fg as u32;
    let bg = bg as u32;
    crate::drivers::framebuffer::draw_char_at(row, col, ch, fg, bg);
    0
}

fn sys_chdir(path_ptr: *const u8) -> u64 {
    let path = match read_c_string(path_ptr) {
        Some(p) => p,
        None => return !0,
    };

    match crate::services::vfs::process_request(crate::ipc::message::FSRequest::ChangeDir { path }) {
        crate::ipc::message::FSResponse::Success => 0,
        _ => !0,
    }
}

fn sys_getcwd(buf: *mut u8, len: usize) -> u64 {
    if buf.is_null() || len == 0 {
        return !0;
    }

    let cwd = match crate::services::vfs::process_request(crate::ipc::message::FSRequest::GetCwd) {
        crate::ipc::message::FSResponse::Cwd(path) => path,
        _ => return !0,
    };

    write_user_string(buf, len, &cwd)
}

fn sys_listdir(path_ptr: *const u8, buf: *mut u8, len: usize) -> u64 {
    if buf.is_null() || len == 0 {
        return !0;
    }

    let path = match read_c_string(path_ptr) {
        Some(p) => p,
        None => return !0,
    };

    let listing = match crate::services::vfs::process_request(crate::ipc::message::FSRequest::ListDir { path }) {
        crate::ipc::message::FSResponse::DirListing(entries) => entries.join("\n"),
        _ => return !0,
    };

    write_user_string(buf, len, &listing)
}

fn sys_uptime() -> u64 {
    crate::drivers::timer::get_uptime_ms()
}

fn sys_shutdown() -> u64 {
    crate::power::shutdown();
    0
}

fn sys_reboot() -> u64 {
    crate::power::reboot();
    0
}

fn write_user_string(dst: *mut u8, len: usize, s: &str) -> u64 {
    let bytes = s.as_bytes();
    let max = len.saturating_sub(1);
    let to_copy = core::cmp::min(bytes.len(), max);
    unsafe {
        let out = core::slice::from_raw_parts_mut(dst, len);
        out[..to_copy].copy_from_slice(&bytes[..to_copy]);
        out[to_copy] = 0;
    }
    to_copy as u64
}

fn exec_user_path(path: &str) -> Result<(), &'static str> {
    use alloc::vec::Vec;
    use crate::task::scheduler::SCHEDULER;

    let mut handle = crate::services::vfs::open(path, 0).map_err(|_| "open failed")?;
    let mut data = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let read = handle.read(&mut buf).map_err(|_| "read failed")?;
        if read == 0 {
            break;
        }
        data.extend_from_slice(&buf[..read]);
    }

    let load = crate::loader::elf::load_user_elf(&data)?;

    let entry = load.entry;
    let user_stack = load.user_stack;
    let addr_space = load.address_space;
    let cr3 = addr_space.cr3.as_u64();

    let mut scheduler = SCHEDULER.lock();
    let current = scheduler.current_task_mut().ok_or("no current task")?;

    current.user_stack = user_stack;
    current.page_table = cr3;
    current.address_space = Some(addr_space);

    unsafe { crate::arch::x86_64::enter_user_mode_with_cr3(entry, user_stack, cr3); }
}

fn spawn_worker() -> ! {
    loop {
        let path = SPAWN_QUEUE.lock().pop();
        if let Some(path) = path {
            let _ = crate::shell::exec_path(&path);
        } else {
            x86_64::instructions::hlt();
        }
    }
}

fn read_c_string(ptr: *const u8) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    const MAX_LEN: usize = 1024;
    let mut bytes = Vec::new();
    unsafe {
        for i in 0..MAX_LEN {
            let b = *ptr.add(i);
            if b == 0 {
                break;
            }
            bytes.push(b);
        }
    }

    String::from_utf8(bytes).ok()
}
