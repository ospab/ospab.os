# ospabOS Audit (Commercial Readiness)

Date: February 6, 2026

This audit compares the current kernel implementation to the architectural docs (notably v0.1.0 Foundation) and evaluates commercial readiness. It focuses on userland, VFS, power, safety, privilege separation, and syscall surface.

## Executive Summary

ospabOS has a solid boot and kernel foundation, but the “userland” is still mostly kernel-resident. The shell is not a true process, signals are not kernel-managed, the VFS is a concrete in-memory tree (not a trait-based abstraction), and syscalls are mostly stubs that route back into kernel helpers. The codebase is structurally coherent and a good learning OS, but it is not yet a commercial OS foundation. Critical gaps are user-mode execution, an ELF loader, real file descriptors, and a preemptive scheduler that performs real context switches.

## 1) Userland & Shell

**Shell is not a standalone process.**
- The shell logic is a function that executes commands inside the kernel. It resolves /bin names and dispatches directly to in-kernel coreutils or the VFS ([kernel/src/shell/mod.rs](kernel/src/shell/mod.rs#L1-L200)).
- There is a `shell_task` stub, but it only echoes keys and does not integrate the actual shell logic or IPC ([kernel/src/shell/task.rs](kernel/src/shell/task.rs#L1-L40)).

**Paths and PATH support are minimal.**
- Command lookup is hardcoded to `/bin/<cmd>` when no slash is present; no environment PATH exists ([kernel/src/shell/mod.rs](kernel/src/shell/mod.rs#L43-L63)).
- Relative paths using `./` are supported only because the shell treats any command containing `/` as a direct path ([kernel/src/shell/mod.rs](kernel/src/shell/mod.rs#L43-L63)), but there is no execution model beyond scripts.

**Ctrl+C is not a signal.**
- Ctrl+C is turned into a control character and handled by the keyboard driver to clear input and reprint the prompt ([kernel/src/drivers/keyboard.rs](kernel/src/drivers/keyboard.rs#L280-L360)).
- There is no process signal delivery (no SIGINT queue, no process groups).

**Reality vs docs:** Docs describe shell as a task and scheduling integration, but actual shell execution is still kernel-direct, and the task stub does not run the command interpreter.

## 2) VFS (Virtual File System)

**VFS is concrete, not a trait.**
- The VFS is a concrete `VFSService` built around `VNode` tree with in-memory data and a `Mutex` root ([kernel/src/services/vfs.rs](kernel/src/services/vfs.rs#L1-L140)).
- There is no trait abstraction for multiple filesystem backends.

**Initrd TAR parsing is basic but works.**
- TAR parser handles ustar-like headers and supports name/prefix, size, and directories ([kernel/src/fs/tar.rs](kernel/src/fs/tar.rs#L1-L120)).
- It does not validate checksums or handle edge cases (pax, long names). This is acceptable for initrd, but not for hardened use.
- Adding a new file to the initrd tar does not require kernel changes as long as the module filename ends with `.tar` ([kernel/src/services/vfs.rs](kernel/src/services/vfs.rs#L170-L235)).

**File descriptors exist only as a global stub.**
- There is a global `FD_TABLE`, but `read` ignores it and only reads from keyboard; `write` ignores it and writes to framebuffer ([kernel/src/syscall/mod.rs](kernel/src/syscall/mod.rs#L1-L140)).
- No `close`, no per-process file table, no file cursor per process. `open` copies the entire file into memory ([kernel/src/syscall/mod.rs](kernel/src/syscall/mod.rs#L140-L210)).

**Reality vs docs:** VFS write support is implemented, but it is still in-memory and not backed by a real device. No abstraction layer for multiple FS backends exists yet.

## 3) Power and CPU Idle

**Reboot/shutdown are implemented via ports.**
- Shutdown uses QEMU/Bochs ACPI ports 0x604 and 0xB004, then triple-fault fallback ([kernel/src/power.rs](kernel/src/power.rs#L1-L80)).
- Reboot uses 0x64 (keyboard controller) with 0xFE ([kernel/src/power.rs](kernel/src/power.rs#L80-L120)).
- There is no ACPI table parsing, no S5 discovery, and no graceful power state transition.

**Idle uses HLT.**
- The idle task uses `hlt` in a loop, which is correct for power/CPU usage ([kernel/src/task/pcb.rs](kernel/src/task/pcb.rs#L84-L100)).

## 4) System Programming Standards

**Unsafe usage is still present where needed.**
- Limine requests and low-level hardware access use unsafe blocks (expected).
- There are still global mutable structures protected by spinlocks (acceptable for a kernel), but allocator behavior and interrupt-safety still need review beyond this audit.

**Privilege separation is not implemented.**
- The scheduler supports address spaces, but tasks run in ring 0 and no ring 3 transition is implemented. GDT/TSS are configured for kernel work; user mode is not active yet.
- Syscall entry point currently halts in a loop, i.e., not a true syscall path ([kernel/src/syscall/mod.rs](kernel/src/syscall/mod.rs#L34-L80)).

**POSIX compatibility is superficial.**
- Syscall names include `open`, `read`, `write`, `exec`, but semantics do not match POSIX (no fd-based read/write, no exec of ELF, no fork/exec separation) ([kernel/src/syscall/mod.rs](kernel/src/syscall/mod.rs#L1-L210)).

## Verdict: Foundation vs Workarounds

**Reliable foundation:**
- Boot, basic scheduler structure, idle `hlt`, PS/2 keyboard path, and kernel-level VFS tree.
- Initrd TAR loading is a practical foundation for early userland bootstrapping.

**Still a workaround / prototype:**
- Shell and coreutils run in kernel context, not in userland.
- Signals are purely input handling (no process signaling model).
- VFS is a concrete tree, not a pluggable interface. No persistent storage.
- Syscalls are stubs and not safe/complete for untrusted user programs.

## Critical Deltas to Become a Commercial OS

1. **User mode execution (Ring 3):**
   - Implement user-mode ELF loader and entry, with proper page tables and privilege transitions.
2. **Real syscalls:**
   - Replace syscall stubs with context capture and dispatch; implement fd-based I/O and `close`, `dup`, `ioctl` as needed.
3. **Process model:**
   - Process creation (`fork/exec` or spawn+exec), process table, signal delivery, and per-process fd tables.
4. **VFS abstraction:**
   - Trait-based filesystem interface with at least one real backend (initrd, then FAT/ext2).
5. **Power management:**
   - ACPI parsing (FADT/DSDT) for real hardware power-off, not only QEMU ports.
6. **Security posture:**
   - Audit `unsafe` and make allocator interrupt-safe; avoid allocations in ISR.

---

If you want, I can follow up with a concrete implementation plan for the top 3 items (Ring 3 + ELF loader + syscall ABI) with code scaffolding and integration steps.