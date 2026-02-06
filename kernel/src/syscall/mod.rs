//! Syscall Interface for ospabOS v0.1.0
//! Implements x86_64 syscall/sysret mechanism

use crate::task::scheduler::SCHEDULER;
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use core::sync::atomic::{AtomicBool, Ordering};

pub mod dispatcher;
pub mod abi;

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
}

struct OpenFile {
    path: String,
    offset: usize,
    data: Vec<u8>,
}

static FD_TABLE: Mutex<Vec<Option<OpenFile>>> = Mutex::new(Vec::new());
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
    
    // Set STAR MSR (CS/SS for syscall/sysret)
    // STAR[63:48] = Kernel CS (0x08)
    // STAR[47:32] = User CS (0x18)
    Msr::new(0xC0000081).write((0x13 << 48) | (0x08 << 32));
    
    // Set LSTAR MSR (syscall entry point)
    Msr::new(0xC0000082).write(syscall_handler as *const () as u64);
    
    // Set SFMASK MSR (RFLAGS mask)
    Msr::new(0xC0000084).write(0x200); // Clear IF
    
    // Enable SYSCALL in EFER
    let mut efer = Efer::read();
    efer |= EferFlags::SYSTEM_CALL_EXTENSIONS;
    Efer::write(efer);
}

/// Syscall handler entry point (called from assembly stub)
#[no_mangle]
pub extern "C" fn syscall_handler() -> ! {
    // This will be implemented with proper context saving
    // For now, just yield
    loop {
        x86_64::instructions::hlt();
    }
}

/// Dispatch syscall from user space
pub fn dispatch_syscall(num: u64, arg1: u64, arg2: u64, _arg3: u64, _arg4: u64) -> u64 {
    match num {
        0 => sys_yield(),
        1 => sys_spawn(arg1 as *const u8, arg2 as usize),
        2 => sys_write(arg1 as *const u8, arg2 as usize),
        3 => sys_read(arg1 as *mut u8, arg2 as usize),
        4 => sys_exit(arg1 as i32),
        5 => sys_getpid(),
        6 => sys_malloc(arg1 as usize), // New: memory allocation
        7 => sys_open(arg1 as *const u8, arg2),
        8 => sys_exec(arg1 as *const u8),
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

fn sys_write(buf: *const u8, len: usize) -> u64 {
    use crate::drivers::framebuffer;
    
    if buf.is_null() {
        return !0;
    }
    
    unsafe {
        let slice = core::slice::from_raw_parts(buf, len);
        if let Ok(s) = core::str::from_utf8(slice) {
            framebuffer::print(s);
            len as u64
        } else {
            !0
        }
    }
}

fn sys_read(buf: *mut u8, len: usize) -> u64 {
    if buf.is_null() || len == 0 {
        return 0;
    }

    if let Some(ch) = crate::drivers::keyboard::try_read_key() {
        unsafe {
            *buf = ch as u8;
        }
        return 1;
    }

    0
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

    let response = crate::services::vfs::process_request(
        crate::ipc::message::FSRequest::ReadFile { path: path.clone() }
    );

    let data = match response {
        crate::ipc::message::FSResponse::FileData(data) => data,
        _ => return !0,
    };

    let mut table = FD_TABLE.lock();
    if table.is_empty() {
        table.resize(3, None);
    }

    let fd = table.iter().position(|e| e.is_none()).unwrap_or(table.len());
    if fd == table.len() {
        table.push(Some(OpenFile { path, offset: 0, data }));
    } else {
        table[fd] = Some(OpenFile { path, offset: 0, data });
    }

    fd as u64
}

fn sys_exec(path_ptr: *const u8) -> u64 {
    let path = match read_c_string(path_ptr) {
        Some(p) => p,
        None => return !0,
    };

    match crate::shell::exec_path(&path) {
        Ok(_) => 0,
        Err(_) => !0,
    }
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
