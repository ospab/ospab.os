# ospabOS Kernel Architecture Review

## Overview
ospabOS v0.33 - microkernel operating system with message-passing IPC architecture. Fully working keyboard input with command history, cursor navigation, and modular service-oriented design.

## Version History

### Version 31 (Stable)
- ✅ Production-ready GDT/TSS/IDT with `spin::Lazy`
- ✅ Atomic ring buffer for keyboard scancodes (lock-free ISR)
- ✅ 5-command history with UP/DOWN arrow navigation
- ✅ Fully working keyboard input (`HandleControl::Ignore`)
- ✅ Removed all blocking `spin_loop()` calls
- ✅ Committed to git

### Version 32
- ✅ LEFT/RIGHT arrow cursor navigation
- ✅ Insert characters at cursor position
- ✅ Backspace deletes at cursor
- ✅ Screen redraw on cursor changes

### Version 33 (Current - Microkernel IPC)
- ✅ Message Bus with service queues
- ✅ Terminal Service wrapping stable I/O
- ✅ VFS Service for filesystem operations
- ✅ Shell command interpreter
- ✅ Modular architecture (ipc/, services/, shell/)

### Version 34
- ✅ Fixed `clear` command deadlock (direct framebuffer call)

### Version 35 (Current - UEFI + Initrd)
- ✅ Limine Memory Map based heap allocation
- ✅ UEFI Framebuffer support (RGB/BGR auto-detection)
- ✅ Serial port (COM1) logger for hardware debugging
- ✅ Initrd-based VFS (Limine modules)
- ✅ `cat` command to read files from initrd
- ✅ Real filesystem with test.txt and README.md

## Microkernel Architecture

### Message Passing IPC

**Location**: `src/ipc/message.rs`

Message enum hierarchy for inter-service communication:
- `Message::FS(FSRequest)` - filesystem operations
- `Message::UI(UIRequest)` - terminal I/O requests
- `Message::Pkg(PkgRequest)` - package management
- `Message::System(SystemRequest)` - system control

**FSRequest variants**:
- `ListDir(path)` → `FSResponse::DirList(Vec<String>)`
- `ReadFile(path)` → `FSResponse::FileContent(Vec<u8>)`
- `WriteFile(path, data)` → `FSResponse::Success`
- `ChangeDir(path)` → `FSResponse::Success/Error`
- `GetCurrentDir()` → `FSResponse::CurrentPath(String)`

**UIRequest variants**:
- `Print(text)` - output to terminal
- `Clear` - clear screen
- `SetColor(r, g, b)` - change text color

**PkgRequest variants**:
- `Install(name)` - install package
- `Remove(name)` - uninstall package
- `List` - list installed packages

### Message Bus

**Location**: `src/ipc/bus.rs`

Central dispatcher with dedicated service queues:
```rust
pub struct MessageBus {
    vfs_queue: VecDeque<FSRequest>,
    ui_queue: VecDeque<UIRequest>,
    pkg_queue: VecDeque<PkgRequest>,
    system_queue: VecDeque<SystemRequest>,
}
```

**Global instance**: `BUS` (spin::Mutex-protected)

**Key methods**:
- `init()` - initialize bus
- `dispatch(Message)` - route message to appropriate queue
- `poll_vfs/ui/pkg/system()` - retrieve queued requests

### Services

#### Terminal Service
**Location**: `src/services/terminal.rs`

Wraps existing stable framebuffer and keyboard I/O **without modification**.

**CRITICAL**: Does NOT rewrite `framebuffer::print()` or `keyboard::process_scancodes()` internals. Uses existing functions directly.

**Key functions**:
- `init()` - initialize service
- `process(UIRequest)` - handle UI requests from Message Bus
- `poll_keyboard()` - poll keyboard input, execute commands via Shell
- Global instance: `TERMINAL` (spin::Mutex)

**Design philosophy**: Thin wrapper preserving all existing stable I/O logic from versions 31-32.

#### VFS Service
**Location**: `src/services/vfs.rs`

Virtual filesystem service with Initrd support (Limine modules).

**State**:
- `current_dir: String` - tracks current working directory
- `files: Vec<FileEntry>` - loaded files from initrd

**Initialization**:
- Parses Limine Module Request
- Loads all module files into memory
- Extracts filenames from module paths
- Stores file address and size for direct access

**Key functions**:
- `init()` - load files from Limine modules
- `process(FSRequest) → FSResponse` - handle filesystem requests
- Implements: `ListDir`, `ReadFile`, `ChangeDir`, `GetCwd`
- `WriteFile` returns error (read-only initrd)

**Current implementation**: Flat filesystem with files from Limine modules
- test.txt - Welcome message with feature list
- README.md - Project documentation

**File access**: Zero-copy reads from module memory regions

### Shell Module

**Location**: `src/shell/mod.rs`

Command interpreter that parses user input and dispatches to services via Message Bus.

**Commands**:
- `help` - show available commands
- `clear` - clear screen (via framebuffer directly)
- `echo <text>` - print text
- `uptime` - show system uptime
- `version` - show kernel version
- `history` - show command history (via keyboard driver)
- `ls` - list directory (via VFS Service from initrd)
- `cat <file>` - read and display file contents from initrd
- `cd <dir>` - change directory (via VFS Service)
- `pwd` - print working directory (via VFS Service)
- `tomato` - package manager placeholder

**Key function**:
- `execute_command(input: &str)` - parse and execute command

**Integration**: 
- Called from `keyboard::execute_command_impl()`
- Uses `services::vfs::VFSSERVICE` for filesystem operations
- Uses `framebuffer::print()` for output (via Terminal Service wrapper)

## Technical Architecture

### Hardware Support (v0.35)

**UEFI Compatibility**:
- Limine bootloader with UEFI support
- Automatic RGB/BGR pixel format detection
- Dynamic framebuffer configuration from Limine
- Proper pitch/stride handling for different resolutions

**Memory Management (Dynamic)**:
- Parses Limine Memory Map at boot
- Uses only USABLE memory regions for heap
- Skips regions below 1MB (legacy hardware)
- Heap size: dynamically allocated (up to 32MB cap)
- Higher Half Direct Map (HHDM) for physical memory access

**Serial Port Debugging**:
- COM1 (0x3F8) serial output
- 38400 baud, 8N1 configuration
- Logs kernel initialization and errors
- Critical for debugging on real hardware where screen fails

### Memory Management
- Custom allocator with linked list algorithm
- Allocation size: 16MB reserved for heap
- No unsafe static mut globals (all use `spin::Lazy` or `spin::Mutex`)

### Interrupt Handling

**GDT (Global Descriptor Table)**
- Location: `src/gdt.rs`
- Kernel code segment (0x08)
- Kernel data segment (0x10) - used for SS register
- TSS with dedicated Double Fault stack (IST[0], 20KB)

**IDT (Interrupt Descriptor Table)**
- Location: `src/interrupts.rs`
- Handlers: Timer (32), Keyboard (33), Double Fault (8)
- Timer uses AtomicU64 for lock-free tick counting
- Keyboard ISR writes to atomic ring buffer (no locks in ISR)

### Keyboard Driver

**Location**: `src/drivers/keyboard.rs`

**Architecture**:
- Atomic ring buffer: 128 scancodes (lock-free ISR)
- 5-command history with UP/DOWN navigation
- LEFT/RIGHT arrow cursor movement with inline editing
- Insert at cursor, backspace deletes at cursor
- `HandleControl::Ignore` (CRITICAL: MapLettersToUnicode breaks input)

**Key functions**:
- `handle_scancode(code: u8)` - ISR writes to ring buffer
- `process_scancodes()` - main loop reads buffer, processes keys
- `handle_arrow_up/down()` - history navigation
- `handle_arrow_left/right()` - cursor navigation
- `execute_command_impl(cmd: &str)` - delegates to `shell::execute_command()`
- `print_history()` - exported for shell module

**Bug fixes**:
- ❌ Version 8-19: `spin_loop(200000/10000)` blocked scancode processing
- ❌ Version 20-23: `HandleControl::MapLettersToUnicode` broke key processing
- ✅ Version 24+: `HandleControl::Ignore` + removed all spin_loop = fully working

### Framebuffer Driver
- Location: `src/drivers/framebuffer.rs`
- Double-buffering with screen scrolling
- Color support (RGB888)
- Character rendering with line wrapping
- Used by Terminal Service **without modification**

## Build System
- Custom target: `x86_64-ospab.json` (soft-float, SSE/SSE2)
- Limine bootloader v10.6.3 (BIOS + UEFI mode)
- Build script: `build_with_alloc.sh` → creates `ospab-os-35.iso`
- ISO size: 1.3MB
- Initrd: Files from `kernel/initrd/` directory included as Limine modules
- Serial output: Redirected to stdio in QEMU (`-serial stdio`)

## Module Structure

```
src/
├── lib.rs              # Module declarations
├── main.rs             # Kernel entry, IPC initialization
├── boot/               # Limine boot protocol
│   └── limine.rs       # Memory Map, Framebuffer, Module requests
├── drivers/
│   ├── keyboard.rs     # PS/2 keyboard with history/cursor
│   ├── framebuffer.rs  # UEFI framebuffer (RGB/BGR auto)
│   ├── serial.rs       # COM1 serial port logger
│   └── timer.rs        # PIT timer
├── gdt.rs              # Global Descriptor Table
├── interrupts.rs       # Interrupt Descriptor Table
├── mm/
│   └── heap_allocator.rs  # Dynamic heap from Limine Memory Map
├── process.rs          # Process structures
├── ipc/
│   ├── mod.rs          # IPC module exports
│   ├── message.rs      # Message enum hierarchy
│   └── bus.rs          # MessageBus dispatcher
├── services/
│   ├── mod.rs          # Services module exports
│   ├── terminal.rs     # TerminalService (I/O wrapper)
│   └── vfs.rs          # VFSService (Initrd filesystem)
└── shell/
    └── mod.rs          # Shell command interpreter

initrd/                 # Files loaded as Limine modules
├── test.txt            # Welcome message
└── README.md           # Project documentation
```

## Critical Design Constraints

### DO NOT MODIFY
1. **Keyboard I/O logic** in `drivers/keyboard.rs` (atomic buffer, history, cursor)
2. **Framebuffer I/O logic** in `drivers/framebuffer.rs` (double-buffering, scrolling)
3. **HandleControl::Ignore** - changing to MapLettersToUnicode breaks keyboard
4. **No spin_loop()** in keyboard processing - causes system hang

### Terminal Service Wrapper
The Terminal Service (`services/terminal.rs`) is a **thin wrapper** that:
- Calls existing `framebuffer::print()` functions
- Calls existing `keyboard::process_scancodes()` functions
- Does NOT reimplement any I/O logic
- Preserves all stability from versions 31-32

This architecture allows modular message-passing while keeping stable I/O code unchanged.

## Next Steps

### Immediate (v36)
- [ ] Test on real UEFI hardware
- [ ] Verify serial logging on physical machine
- [ ] Test cat command with different file types
- [ ] Add more files to initrd

### Short-term (v37-38)
- [ ] Implement proper physical memory manager
- [ ] Add paging support (4-level page tables)
- [ ] Package manager service implementation
- [ ] System service (reboot, shutdown)
- [ ] Write support (RAM disk)

### Long-term
- [ ] Real filesystem driver (FAT32/ext2)
- [ ] Multi-process support with IPC
- [ ] User-space programs
- [ ] Dynamic module loading
- [ ] Network stack

## Known Issues

### Resolved
- ✅ Keyboard input blocking (removed spin_loop)
- ✅ HandleControl mapping (switched to Ignore)
- ✅ Static mut safety (replaced with spin::Lazy)
- ✅ History navigation (UP/DOWN arrows)
- ✅ Cursor editing (LEFT/RIGHT arrows)

### Current
- None - version 35 compiles successfully with UEFI support

## Testing Status
- [x] Version 31 - TESTED, WORKING, COMMITTED
- [x] Version 32 - BUILT SUCCESSFULLY (cursor navigation)
- [x] Version 33 - BUILT SUCCESSFULLY (microkernel IPC)
- [x] Version 34 - BUILT SUCCESSFULLY (fixed clear command)
- [x] Version 35 - BUILT SUCCESSFULLY (UEFI + Initrd - ready for testing)

## Hardware Compatibility

### Tested Platforms
- ✅ QEMU (x86_64) - BIOS mode
- ⏳ QEMU (x86_64) - UEFI mode (pending)
- ⏳ Real hardware with UEFI (pending)

### Known Requirements
- x86_64 CPU with SSE/SSE2
- At least 128MB RAM (recommended)
- UEFI firmware (for full feature support)
- Serial port for hardware debugging (optional)

## Commit Log
- v31: "Production kernel with stable keyboard, history, and cursor navigation"
- v34: "Fixed clear command deadlock"
- v35: (pending) "UEFI support with Limine Memory Map and Initrd VFS"

---

**Last updated**: Version 35 build  
**Status**: Ready for UEFI hardware testing  
**Build**: ospab-os-35.iso (1.3MB)

## Changes Summary (v33 → v35)

### boot/limine.rs
- Added `LimineFile` structure
- Added `ModuleRequest` and `ModuleResponse`
- Added `modules()` iterator for accessing initrd files
- Added `get_module()` and `module_count()` helpers

### mm/heap_allocator.rs
- Removed static `HEAP_SPACE` array
- Implemented dynamic heap allocation from Limine Memory Map
- Searches for largest USABLE memory region (>16MB)
- Uses HHDM offset for physical memory access
- Caps heap at 32MB for safety

### drivers/framebuffer.rs
- Added pixel format detection fields (red/green/blue shift)
- Added `is_bgr` flag for BGR format detection
- Implemented automatic RGB↔BGR conversion in `put_pixel()`
- Reads mask shift values from Limine framebuffer info

### drivers/serial.rs (NEW)
- COM1 (0x3F8) serial port driver
- 38400 baud, 8N1 configuration
- `init()`, `write()`, `log()`, `error()`, `info()`, `debug()` functions
- Macros: `serial_print!()`, `serial_println!()`

### services/vfs.rs
- Complete rewrite for Initrd support
- Added `FileEntry` structure (name, address, size)
- Loads files from Limine modules at init
- Implements zero-copy file reads
- Read-only filesystem (initrd)

### shell/mod.rs
- Added `cat` command for file reading
- Updated help text to mention cat and initrd
- File content display with UTF-8 validation

### main.rs
- Added serial port initialization
- Updated welcome screen for v0.35
- Changed step counter from 7 to 8

### limine.conf
- Added module_path entries for test.txt and README.md
- Updated boot entry title to "v0.35 - UEFI + Initrd"

### build_with_alloc.sh
- Added initrd directory creation in ISO
- Copies files from kernel/initrd/ to iso_root/initrd/

### initrd/ (NEW)
- test.txt - Welcome message with feature list
- README.md - Project documentation
