# ospabOS

**A preemptive multitasking microkernel operating system written in Rust**

Version: **v0.1.5** | Architecture: x86_64 | Bootloader: Limine 7.x

[![License](https://img.shields.io/badge/license-Educational-blue.svg)]()
[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)]()
[![Platform](https://img.shields.io/badge/platform-x86__64-green.svg)]()
[![Status](https://img.shields.io/badge/status-active-green.svg)]()

## Overview

ospabOS v0.1.5 is a bare-metal hybrid bootable microkernel with syscalls, virtual memory management, and process isolation. Boots on both QEMU and real x86_64 hardware.

### Key Features (v0.1.5)

- ✅ **Preemptive Multitasking** - Round-Robin scheduler with context switching
- ✅ **System Calls** - x86_64 syscall/sysret interface (yield, spawn, write, read, exit, getpid, **malloc**)
- ✅ **Process Launch** - open/exec syscall stubs and spawn worker (kernel tasks)
- ✅ **Virtual Memory Manager** - 4-level paging, user/kernel address space separation
- ✅ **Dynamic Memory** - sys_malloc for userland heap allocation
- ✅ **Process Isolation** - CR3 switching per task, protected address spaces
- ✅ **Physical Memory Manager** - Frame allocator (128 MB, 4 KB frames)
- ✅ **User Stacks** - Per-task stack isolation
- ✅ **Task State Segment** - 20 KB interrupt stack
- ✅ **Unix-like VFS** - Hierarchical filesystem (`/bin`, `/etc`, `/home`, `/dev`)
- ✅ **Initrd TAR Support** - ustar parser for initrd-backed files
- ✅ **Bare Metal Boot** - Hybrid BIOS/UEFI ISO with dual EFI paths
- ✅ **Hardware Compatibility** - Tests pass on QEMU, Bochs, and real laptops
- ✅ **Grape Text Editor** - nano-inspired editor with syscall integration
- ✅ **Serial Console** - COM1 debugging (115200 baud)
- ✅ **Keyboard Input** - PS/2 driver with IRQ1 handling
- ✅ **Command Shell** - /bin lookup, #! script support, Ctrl+C cancel
- ✅ **Coreutils (in-kernel)** - ls, cat, mkdir, cp, mv
- ✅ **Power Management** - shutdown/reboot commands
- ⏳ **DOOM** - Runs as separate process with sys_malloc framebuffer


## Quick Start

### Running in QEMU

```bash
cd kernel
wsl bash build.sh
cd isos
# Graphical window (keyboard works here)
qemu-system-x86_64 -cdrom ospab-os-71.iso -m 256M

# Serial output (no keyboard)
qemu-system-x86_64 -cdrom ospab-os-71.iso -m 256M -serial stdio
```

**Commands to try:**
- `help` - Show available commands
- `version` - Show OS version
- `ls` - List files in current directory
- `cat test.txt` - Read a file
- `grape <file>` - Edit file with Grape editor
- `mkdir /tmp/demo` - Create a directory
- `cp /etc/hostname /tmp/host` - Copy a file
- `mv /tmp/host /tmp/host.bak` - Move a file
- `doom` - Launch DOOM (uses sys_malloc for framebuffer)
- `shutdown` - Poweroff system
- `reboot` - Restart system

### Building from Source

**Requirements:**
- Rust nightly toolchain with x86_64 target
- WSL/Linux for build script (or native bash)
- xorriso ≥1.5.6 for ISO creation
- mtools for FAT ESP image support
- Limine v7.x bootloader (included in `kernel/tools/`)

**Build Steps:**

```bash
cd kernel

# Compile kernel and dependencies
cargo +nightly build --release -Z build-std=core,alloc --target x86_64-ospab.json

# Create hybrid ISO (BIOS + UEFI)
bash build.sh
```

**Output:** `kernel/isos/ospab-os-71.iso` (22 MB)

## System Architecture

### v0.1.0 "Foundation" Components

```
ospabOS v0.1.0
├── Kernel (kernel/src/)
│   ├── task/          - Multitasking subsystem
│   │   ├── pcb.rs     - Process Control Block
│   │   ├── scheduler.rs - Round-Robin scheduler
│   │   ├── tss.rs     - Task State Segment (IST)
│   │   └── context.rs - Context switching (naked_asm)
│   ├── syscall/       - System call interface
│   │   ├── mod.rs     - Dispatcher
│   │   ├── abi.rs     - ABI definitions
│   │   └── handlers.rs - Syscall implementations
│   ├── fs/            - TAR initrd helpers
│   ├── apps/          - Coreutils stubs
│   ├── mem/           - Memory management
│   │   └── physical.rs - Frame allocator (128 MB)
│   ├── drivers/       - Hardware drivers
│   │   ├── framebuffer.rs  - VGA/UEFI display
│   │   ├── keyboard.rs     - PS/2 driver (IRQ1)
│   │   ├── timer.rs        - PIT timer (100 Hz)
│   │   └── serial.rs       - COM1 serial port
│   ├── services/      - System services
│   │   ├── vfs.rs     - Virtual filesystem
│   │   └── terminal.rs - Terminal service
│   ├── grape/         - Text editor module
│   ├── shell/         - Command interpreter
│   │   └── task.rs    - Shell as task
│   ├── doom/          - DOOM port (WIP)
│   │   └── task.rs    - DOOM as task
│   └── interrupts.rs  - IDT, GDT, PIC setup
├── Boot (Limine 7.x)  - BIOS/UEFI bootloader
└── Initrd             - Initial ramdisk files
```

### v0.1.5 Architecture

```
ospabOS v0.1.5 - Bare Metal + Syscalls + VMM
├── Kernel (kernel/src/)
│   ├── main.rs            - Entry point, subsystem initialization
│   ├── boot.rs            - Limine protocol, HHDM offset
│   ├── gdt.rs             - Global Descriptor Table
│   ├── interrupts.rs      - IDT, exception handlers, PIC setup
│   │
│   ├── task/              - Process & Scheduling subsystem
│   │   ├── pcb.rs         - Process Control Block (PID, stack, address_space)
│   │   ├── scheduler.rs   - Round-Robin + CR3 switching per task
│   │   ├── context.rs     - Context save/restore (naked asm)
│   │   └── tss.rs         - Task State Segment (IST for interrupts)
│   │
│   ├── mem/               - Memory Management (Physical + Virtual)
│   │   ├── physical.rs    - Frame allocator (128 MB, bitmap-based)
│   │   └── vmm.rs         - Virtual Memory Manager
│   │       ├── 4-level paging (PML4→PDPT→PD→PT)
│   │       ├── User/Kernel address space separation
│   │       ├── CR3 management per process
│   │       └── allocate_pages(), map_page(), unmap_page()
│   │
│   ├── syscall/           - System Call Interface
│   │   ├── mod.rs         - Dispatcher (0=yield, 1=spawn, ..., 6=malloc)
│   │   ├── abi.rs         - ABI wrappers (syscall0-4)
│   │   └── dispatcher.rs  - Route syscalls to handlers
│   │
│   ├── drivers/           - Hardware Drivers
│   │   ├── framebuffer.rs - Graphics output (VGA/UEFI)
│   │   ├── keyboard.rs    - PS/2 keyboard (IRQ1)
│   │   ├── timer.rs       - PIT timer (100 Hz ticks)
│   │   └── serial.rs      - COM1 (115200 baud)
│   │
│   ├── services/          - System Services
│   │   ├── vfs.rs         - Virtual filesystem
│   │   ├── terminal.rs    - Terminal service
│   │   └── message_bus.rs - IPC message passing
│   │
│   ├── grape/             - Text Editor (Syscall-ready)
│   ├── doom/              - DOOM v0.1.5 (uses sys_malloc for framebuffer)
│   │   └── v015.rs        - Refactored for VMM
│   ├── shell/             - Command Interpreter & Dispatcher
│   │   └── mod.rs         - Command execution (doom, shutdown, reboot, etc.)
│   └── power.rs           - Power management (shutdown, reboot)
│
├── Boot (Limine 7.x)
│   ├── BOOTX64.EFI        - UEFI bootloader (in /EFI/BOOT and /boot/EFI/BOOT)
│   ├── limine.conf        - Boot configuration
│   └── limine-bios.sys    - BIOS boot stages
│
└── Initrd                 - Initial ramdisk
    ├── DOOM1.WAD          - DOOM graphics/level data
    └── fonts/             - Font data for text rendering
```

### Memory Layout (v0.1.5)

```
Per-Task Address Space (with 4-level paging):

User Space (0x0000_0000_0000_0000 - 0x0000_7FFF_FFFF_FFFF):  [128 TB]
  ┌─────────────────────────────┐
  │  User Code (loaded)         │  0x0000_0000_0010_0000
  ├─────────────────────────────┤
  │  User Data/BSS              │  
  ├─────────────────────────────┤
  │  Dynamic Heap (sys_malloc)  │  ← DOOM framebuffer allocated here
  ├─────────────────────────────┤  Start: 0x0000_4000_0000_0000
  │                             │
  │  (Free space)               │
  │                             │
  ├─────────────────────────────┤
  │  User Stack (grows down)    │  ← sp register
  └─────────────────────────────┘

Kernel Space (0xFFFF_8000_0000_0000+):  [Upper half via HHDM]
  ┌─────────────────────────────┐
  │  Kernel Code & Rodata       │
  ├─────────────────────────────┤
  │  Kernel Heap (allocator)    │
  ├─────────────────────────────┤
  │  Task State Segment (IST)   │  ← Interrupt stack
  ├─────────────────────────────┤
  │  GDT, IDT, Page Tables      │
  │  Frame Allocator Bitmap     │
  │  Task PCBs                  │
  │  Scheduler state            │
  └─────────────────────────────┘
```

## Filesystem Structure

```
/              Root directory
├── bin/       System commands (ls, cat, grape)
├── etc/       Configuration files
│   ├── hostname       - System hostname
│   └── os-release     - OS version info
├── home/      User directories
│   └── user/          - Default user home
├── dev/       Device files
│   ├── null           - Null device
│   ├── zero           - Zero device
│   ├── keyboard       - Keyboard input
│   ├── framebuffer    - Display device
│   └── serial         - COM1 serial
├── tmp/       Temporary files
├── usr/       User programs
│   └── bin/           - User binaries
└── var/       Variable data
    └── log/           - System logs
```

## Available Commands

| Command | Description | Example |
|---------|-------------|---------|
| `help` | Show available commands | `help` |
| `version` | Display OS version | `version` |
| `ls` | List directory contents | `ls /bin` |
| `cat` | Display file contents | `cat /etc/hostname` |
| `cd` | Change directory | `cd /etc` |
| `pwd` | Print working directory | `pwd` |
| `uptime` | System uptime | `uptime` |
| `clear` | Clear screen | `clear` |
| `doom` | Run DOOM (separate process with sys_malloc) | `doom` |
| `history` | Command history | `history` |
| `grape` | Text editor | `grape test.txt` |
| `shutdown` | Poweroff system | `shutdown` |
| `reboot` | Restart system | `reboot` |

## Grape Text Editor

Nano-inspired text editor with standard keybindings:

| Key | Action |
|-----|--------|
| `Ctrl+G` | Show help |
| `Ctrl+X` | Save file |
| `Ctrl+C` | Exit editor |
| `Ctrl+W` | Search (NYI) |
| `Ctrl+K` | Cut line (NYI) |
| `Ctrl+U` | Paste (NYI) |
| `Arrow Keys` | Navigate cursor |
| `Page Up/Down` | Scroll 10 lines |
| `Home/End` | Jump to line start/end |

**Usage:**
```bash
[ospab]~> grape myfile.txt
# Edit text with arrow keys
# Ctrl+X to save, Ctrl+C to exit
```

## Installation

See [BARE_METAL_GUIDE.md](docs/BARE_METAL_GUIDE.md) for detailed installation instructions for:
- USB boot media creation
- CD/DVD burning
- UEFI/BIOS hybrid boot
- Troubleshooting
- Hardware compatibility

## Documentation

- **[v015-bare-metal-syscalls.md](docs/v015-bare-metal-syscalls.md)** - v0.1.5 bare metal boot & syscalls (Feb 2, 2026)
- **[serial-stdio-keyboard-issue.md](docs/serial-stdio-keyboard-issue.md)** - QEMU serial vs PS/2 issue (Feb 2, 2026)
- **[fix-keyboard-and-limine.md](docs/fix-keyboard-and-limine.md)** - Keyboard & Limine config fix (Jan 20, 2026)
- **[BARE_METAL_GUIDE.md](docs/BARE_METAL_GUIDE.md)** - Installation guide (USB, CD, UEFI/BIOS)
- **[production-ready.md](production-ready.md)** - Production readiness notes
- **[review.md](review.md)** - Development review
- **Serial Output** - Real-time debug information (COM1, 115200 baud)

## Version History

### v0.1.5 (Current) - February 6, 2026
**Major release: Bare metal boot & syscalls architecture**

✅ **New in v0.1.5:**
- ✅ Bare metal boot fixes (dual EFI paths, config enrollment)
- ✅ Virtual Memory Manager (4-level paging, 128 TB user space)
- ✅ User/kernel address space separation with CR3 switching
- ✅ sys_malloc syscall for dynamic memory allocation
- ✅ DOOM refactored to run as separate process with dynamic framebuffer
- ✅ Power management (shutdown, reboot commands)
- ✅ Enhanced build script for physical hardware compatibility
- ✅ TAR initrd parser + VFS population from initrd
- ✅ Coreutils wrappers (ls, cat, mkdir, cp, mv)
- ✅ Ctrl+C input cancel (no echo)
- ✅ Syscall stubs for open/exec

✅ **From v0.1.0 (inherited):**
- Preemptive multitasking with Round-Robin scheduler
- 6 system calls: yield, spawn, write, read, exit, getpid, **malloc**
- Physical frame allocator (128 MB)
- Task State Segment (IST) with 20 KB interrupt stack
- Shell interpreter with command history
- Keyboard driver (PS/2, IRQ1)
- BIOS+UEFI hybrid ISO

**ISO Details:**
- `ospab-os-71.iso` - 22.7 MB
- Boots on QEMU, Bochs, and real x86_64 hardware
- Hybrid BIOS/UEFI with Limine 7.x bootloader
- EFI System Partition (FAT32, 20 MB)

### v0.1.0 "Foundation" - February 1, 2026
**Milestone: Stable preemptive multitasking kernel**

✅ **Completed:**
- Preemptive multitasking kernel
- 6 syscalls (yield, spawn, write, read, exit, getpid)
- Physical memory allocator (128 MB)
- Context switching with naked_asm
- Shell and DOOM as tasks
- Keyboard fix (hlt instead of spin_loop)
- Limine config fix (syntax and URI)
- BIOS+UEFI hybrid ISO

### Previous Versions

- Unix-like VFS hierarchy
- Grape text editor
- Directory navigation
- Device files (/dev)
## Roadmap

### Near-term (v0.2.0 - v0.3.0)
- [ ] **ELF loader** - Load executables from VFS
- [ ] **User mode syscalls** - Ring 3 execution
- [ ] **DOOM as userspace task** - Run DOOM in isolated address space
- [ ] **Persistent storage** - FAT32/ext2 filesystem driver
- [ ] **AHCI disk driver** - SATA disk access

### Mid-term (v0.4.0 - v0.6.0)
- [ ] **fork/exec** - Full process creation primitives
- [ ] **Pipes and IPC** - Inter-process communication
- [ ] **Full DOOM engine** - doomgeneric port with BSP rendering
- [ ] **Load DOOM1.WAD** - Shareware episode support
- [ ] **Implement search in Grape** - Ctrl+W functionality
- [ ] **Cut/paste in Grape** - Ctrl+K/U keybindings
- [ ] **Basic networking** - E1000 NIC driver
- [ ] **TCP/IP stack** - smoltcp integration

### Long-term (v0.7.0 - v1.0)
- [ ] **SMP support** - Multi-core scheduling
- [ ] **Sound driver** - Sound Blaster 16 / AC'97
- [ ] **GUI subsystem** - Framebuffer compositing
- [ ] **USB stack** - USB 2.0/3.0 support
- [ ] **POSIX compatibility** - Unix-like syscalls
- [ ] **Package manager** - Binary distribution system

## Technical Specifications

### Kernel Features

- **Language**: Rust (nightly) - 100% safe Rust in core kernel
- **Architecture**: x86_64 only (5-level paging support planned)
- **Boot Protocol**: Limine v7.x (BIOS + UEFI hybrid)
- **Memory Management**: 
  - Heap: linked_list_allocator (up to 32 MB)
  - Physical: Frame allocator (128 MB, bitmap-based)
  - Virtual: Page tables (in development)
- **Interrupts**: IDT with dual PIC (8259A)
- **Timer**: PIT at 100 Hz (10ms jiffies)
- **Multitasking**: Preemptive Round-Robin scheduler
- **System Calls**: x86_64 syscall/sysret (yield, spawn, write, read, exit, getpid, malloc, open, exec)
- **IPC**: Message bus microkernel (in development)
- **VFS**: In-memory hierarchical filesystem (initrd-based)

### Hardware Requirements

| Component | Requirement |
|-----------|-------------|
| CPU | x86_64 with SSE2, syscall/sysret support |
| RAM | 256 MB minimum, 512 MB recommended |
| Graphics | VGA (BIOS) or UEFI GOP |
| Keyboard | PS/2 (IRQ1) or USB HID |
| Firmware | UEFI or Legacy BIOS |
| Storage | CD/DVD, USB, or virtual disk |

### Supported Platforms

- ✅ QEMU/KVM (fully tested)
- ✅ VirtualBox (tested)
- ✅ VMware (tested)
- ✅ Physical hardware (UEFI) - tested on real systems
- ✅ Physical hardware (BIOS/CSM) - tested on real systems
- ✅ Hybrid BIOS+UEFI ISO

## License

Educational project - Free to use and modify for learning purposes.

## Credits

- **Bootloader**: Limine v7.x by mintsuki
- **Rust Ecosystem**: x86_64, pc-keyboard, linked_list_allocator, spin
- **Architecture**: Inspired by Linux, MINIX, SerenityOS, and ToaruOS
- **Development**: ospab-projects
- **Documentation**: Production-ready kernel development practices

---

**Current Status:** Active development | ISO #71 stable | Keyboard working | Multitasking operational

---

**Note**: ospabOS is a hobby/learning project and not intended for production use.

Last updated: February 6, 2026
