//! Syscall ABI definitions for userspace

/// Syscall ABI v0.1.0
/// 
/// Calling convention (x86_64 syscall):
/// - RAX: syscall number
/// - RDI: arg1
/// - RSI: arg2
/// - RDX: arg3
/// - R10: arg4
/// - R8:  arg5
/// - R9:  arg6
/// 
/// Return: RAX

/// sys_yield() -> 0
/// Yield CPU to another task
pub const SYS_YIELD: u64 = 0;

/// sys_spawn(entry_point: *const fn(), name: *const u8, name_len: usize) -> pid
/// Spawn a new task
pub const SYS_SPAWN: u64 = 1;

/// sys_write(fd: u32, buf: *const u8, len: usize) -> bytes_written
/// Write to file descriptor (1=stdout, 2=stderr)
pub const SYS_WRITE: u64 = 2;

/// sys_read(fd: u32, buf: *mut u8, len: usize) -> bytes_read
/// Read from file descriptor (0=stdin)
pub const SYS_READ: u64 = 3;

/// sys_exit(code: i32) -> !
/// Terminate current task
pub const SYS_EXIT: u64 = 4;

/// sys_getpid() -> pid
/// Get current process ID
pub const SYS_GETPID: u64 = 5;

/// sys_open(path: *const u8, flags: u64) -> fd
/// Open a file from VFS
pub const SYS_OPEN: u64 = 7;

/// sys_exec(path: *const u8) -> status
/// Execute a script or binary
pub const SYS_EXEC: u64 = 8;

/// sys_draw_char(x: u64, y: u64, ch: u64, fg: u64, bg: u64) -> status
/// Draw a character cell using the kernel terminal renderer
pub const SYS_DRAW_CHAR: u64 = 9;

/// sys_chdir(path: *const u8) -> status
pub const SYS_CHDIR: u64 = 10;

/// sys_getcwd(buf: *mut u8, len: usize) -> bytes_written
pub const SYS_GETCWD: u64 = 11;

/// sys_listdir(path: *const u8, buf: *mut u8, len: usize) -> bytes_written
pub const SYS_LISTDIR: u64 = 12;

/// sys_uptime() -> ms
pub const SYS_UPTIME: u64 = 13;

/// sys_shutdown() -> !
pub const SYS_SHUTDOWN: u64 = 14;

/// sys_reboot() -> !
pub const SYS_REBOOT: u64 = 15;

/// Userspace syscall wrappers (for future userspace programs)
#[allow(dead_code)]
mod userspace {
    use super::*;
    use core::arch::asm;

    #[inline(always)]
    pub unsafe fn syscall0(num: u64) -> u64 {
        let ret: u64;
        asm!(
            "syscall",
            in("rax") num,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }

    #[inline(always)]
    pub unsafe fn syscall1(num: u64, arg1: u64) -> u64 {
        let ret: u64;
        asm!(
            "syscall",
            in("rax") num,
            in("rdi") arg1,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }

    #[inline(always)]
    pub unsafe fn syscall3(num: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
        let ret: u64;
        asm!(
            "syscall",
            in("rax") num,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
        ret
    }

    pub fn yield_cpu() {
        unsafe { syscall0(SYS_YIELD); }
    }

    pub fn write_stdout(msg: &str) -> usize {
        unsafe {
            syscall3(SYS_WRITE, 1, msg.as_ptr() as u64, msg.len() as u64) as usize
        }
    }

    pub fn exit(code: i32) -> ! {
        unsafe {
            syscall1(SYS_EXIT, code as u64);
        }
        loop {}
    }

    pub fn getpid() -> u32 {
        unsafe { syscall0(SYS_GETPID) as u32 }
    }

    pub fn open(path: &str, flags: u64) -> u64 {
        unsafe { syscall3(SYS_OPEN, path.as_ptr() as u64, flags, 0) }
    }

    pub fn exec(path: &str) -> u64 {
        unsafe { syscall1(SYS_EXEC, path.as_ptr() as u64) }
    }

    pub fn draw_char(x: u64, y: u64, ch: u64, fg: u64, bg: u64) -> u64 {
        let ret: u64;
        unsafe {
            asm!(
                "syscall",
                in("rax") SYS_DRAW_CHAR,
                in("rdi") x,
                in("rsi") y,
                in("rdx") ch,
                in("r10") fg,
                in("r8") bg,
                lateout("rax") ret,
                options(nostack, preserves_flags)
            );
        }
        ret
    }

    pub fn chdir(path: &str) -> u64 {
        unsafe { syscall1(SYS_CHDIR, path.as_ptr() as u64) }
    }

    pub fn getcwd(buf: *mut u8, len: usize) -> u64 {
        unsafe { syscall3(SYS_GETCWD, buf as u64, len as u64, 0) }
    }

    pub fn listdir(path: &str, buf: *mut u8, len: usize) -> u64 {
        unsafe { syscall3(SYS_LISTDIR, path.as_ptr() as u64, buf as u64, len as u64) }
    }

    pub fn uptime() -> u64 {
        unsafe { syscall0(SYS_UPTIME) }
    }

    pub fn shutdown() -> ! {
        unsafe { syscall0(SYS_SHUTDOWN); }
        loop {}
    }

    pub fn reboot() -> ! {
        unsafe { syscall0(SYS_REBOOT); }
        loop {}
    }
}
