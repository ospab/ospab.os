//! SYSCALL entry stub and low-level context save/restore.

use core::arch::naked_asm;

const SYSCALL_STACK_SIZE: usize = 4096 * 4;

#[repr(C, align(16))]
struct SyscallStack {
    data: [u8; SYSCALL_STACK_SIZE],
}

static SYSCALL_STACK: SyscallStack = SyscallStack { data: [0; SYSCALL_STACK_SIZE] };

#[no_mangle]
static mut SYSCALL_USER_RSP: u64 = 0;

#[no_mangle]
pub extern "C" fn do_syscall(num: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> u64 {
    crate::syscall::dispatch_syscall(num, arg1, arg2, arg3, arg4, arg5)
        .wrapping_add(0 * arg5)
}

#[unsafe(naked)]
pub unsafe extern "C" fn syscall_handler() -> ! {
    naked_asm!(
        // Save user RSP and switch to kernel syscall stack
        "mov [rip + {user_rsp}], rsp",
        "lea rsp, [rip + {stack_base}]",
        "add rsp, 16384",
        // Save volatile state from user
        "push r11",
        "push rcx",
        "push r9",
        "push r8",
        "push r10",
        "push rdx",
        "push rsi",
        "push rdi",
        "push rbx",
        "push rbp",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        // Arrange arguments for do_syscall
        "mov rdi, rax",         // num
        "mov rsi, [rsp + 48]",  // arg1 (saved rdi)
        "mov rdx, [rsp + 56]",  // arg2 (saved rsi)
        "mov rcx, [rsp + 64]",  // arg3 (saved rdx)
        "mov r8,  [rsp + 72]",  // arg4 (saved r10)
        "mov r9,  [rsp + 80]",  // arg5 (saved r8)
        "call {do_syscall}",
        // Restore registers (except rax which holds return value)
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbp",
        "pop rbx",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop r10",
        "pop r8",
        "pop r9",
        "pop rcx",
        "pop r11",
        // Restore user RSP and return to user
        "mov rsp, [rip + {user_rsp}]",
        "sysretq",
        user_rsp = sym SYSCALL_USER_RSP,
        stack_base = sym SYSCALL_STACK,
        do_syscall = sym do_syscall
    )
}
