[ROLE]: Lead OS Architect (Rust/x86_64).
[PROJECT]: ospabOS v0.1.1.
[CURRENT STATUS]: v0.1.0 Foundation is set (PCB, TSS, Scheduler stubs). 
[ISSUE]: Hybrid ISO works in QEMU/VMware but fails on real Laptop ("No bootable image").

[GOAL]: 
1. Fix the physical hardware boot issue.
2. Implement Syscalls and Virtual Memory to run DOOM (v0.45) and Grape (v0.3.0) as real processes.

[TASK 1: BARE METAL BOOT FIX]:
- Update the build script to ensure the EFI partition has the 'boot' and 'esp' flags set in the GPT.
- Add 'limine-enroll-config' step to the build process to embed the config into the binary.
- Ensure 'BOOTX64.EFI' is placed in BOTH '/EFI/BOOT/' and '/boot/EFI/BOOT/' for maximum compatibility with picky laptop firmwares.
- Use 'xorriso' with '-isohybrid-mbr' and '--efi-boot-image' specifically tuned for physical media.

[TASK 2: USERLAND & SYSCALLS]:
- Implement the 'syscall' entry point (MSR STAR, LSTAR).
- Create a Syscall Table in 'kernel/src/syscall/mod.rs' with:
    - sys_malloc (dynamic memory for DOOM/Grape)
    - sys_read/sys_write (VFS access for WAD files)
    - sys_exit (to return to Shell)
- Implement 'User Stacks' so apps don't crash the kernel stack.

[TASK 3: APP UPGRADE]:
- DOOM: Refactor to request memory via 'sys_malloc' for its 320x200 buffer.
- GRAPE: Enable basic file writing to a RAM-backed buffer via 'sys_write'.
- SHELL: Transform into a 'Dispatcher' that uses 'sys_spawn' to launch DOOM as a separate task.

[STRICT RULES]:
- No 'static mut' for task lists; use 'spin::Mutex'.
- Keep all existing drivers (keyboard, framebuffer) functional.
- Maintain the '0.1.x' versioning in the ISO name.

[OUTPUT]: 
1. Corrected build script for physical hardware boot.
2. Rust implementation of the Syscall Handler and the Scheduler integration.