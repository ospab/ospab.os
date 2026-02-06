use core::arch::asm;

pub const SYS_READ: u64 = 3;
pub const SYS_EXIT: u64 = 4;
pub const SYS_OPEN: u64 = 7;
pub const SYS_EXEC: u64 = 8;
pub const SYS_DRAW_CHAR: u64 = 9;
pub const SYS_CHDIR: u64 = 10;
pub const SYS_GETCWD: u64 = 11;
pub const SYS_LISTDIR: u64 = 12;
pub const SYS_UPTIME: u64 = 13;
pub const SYS_SHUTDOWN: u64 = 14;
pub const SYS_REBOOT: u64 = 15;

pub unsafe fn read(fd: u64, buf: *mut u8, len: usize) -> u64 {
    let ret: u64;
    asm!(
        "syscall",
        in("rax") SYS_READ,
        in("rdi") fd,
        in("rsi") buf,
        in("rdx") len,
        lateout("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
}

pub unsafe fn open(path: *const u8, flags: u64) -> u64 {
    let ret: u64;
    asm!(
        "syscall",
        in("rax") SYS_OPEN,
        in("rdi") path,
        in("rsi") flags,
        lateout("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
}

pub unsafe fn exec(path: *const u8) -> u64 {
    let ret: u64;
    asm!(
        "syscall",
        in("rax") SYS_EXEC,
        in("rdi") path,
        lateout("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
}

pub unsafe fn draw_char(x: u64, y: u64, ch: u64, fg: u64, bg: u64) -> u64 {
    let ret: u64;
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
    ret
}

pub unsafe fn chdir(path: *const u8) -> u64 {
    let ret: u64;
    asm!(
        "syscall",
        in("rax") SYS_CHDIR,
        in("rdi") path,
        lateout("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
}

pub unsafe fn getcwd(buf: *mut u8, len: usize) -> u64 {
    let ret: u64;
    asm!(
        "syscall",
        in("rax") SYS_GETCWD,
        in("rdi") buf,
        in("rsi") len,
        lateout("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
}

pub unsafe fn listdir(path: *const u8, buf: *mut u8, len: usize) -> u64 {
    let ret: u64;
    asm!(
        "syscall",
        in("rax") SYS_LISTDIR,
        in("rdi") path,
        in("rsi") buf,
        in("rdx") len,
        lateout("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
}

pub unsafe fn uptime() -> u64 {
    let ret: u64;
    asm!(
        "syscall",
        in("rax") SYS_UPTIME,
        lateout("rax") ret,
        options(nostack, preserves_flags)
    );
    ret
}

pub unsafe fn shutdown() -> ! {
    asm!(
        "syscall",
        in("rax") SYS_SHUTDOWN,
        options(noreturn)
    );
}

pub unsafe fn reboot() -> ! {
    asm!(
        "syscall",
        in("rax") SYS_REBOOT,
        options(noreturn)
    );
}

pub unsafe fn exit(code: i32) -> ! {
    asm!(
        "syscall",
        in("rax") SYS_EXIT,
        in("rdi") code as u64,
        options(noreturn)
    );
}
