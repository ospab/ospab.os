# ospabOS Architecture Documentation

**Version**: 0.38  
**Last Updated**: February 2, 2026

## Table of Contents

1. [Overview](#overview)
2. [Boot Process](#boot-process)
3. [Kernel Architecture](#kernel-architecture)
4. [Memory Management](#memory-management)
5. [Interrupt Handling](#interrupt-handling)
6. [Device Drivers](#device-drivers)
7. [Filesystem (VFS)](#filesystem-vfs)
8. [IPC & Microkernel](#ipc--microkernel)
9. [Shell & Text Editor](#shell--text-editor)
10. [Development Roadmap](#development-roadmap)

---

## Overview

ospabOS is a microkernel operating system written in Rust for x86_64 architecture. It follows a message-passing IPC model with minimal kernel services and user-space components.

### Design Principles

- **Microkernel**: Minimal kernel, services in user space (planned)
- **Safety**: Rust's memory safety guarantees
- **Unix-like**: Familiar filesystem hierarchy and command interface
- **Educational**: Clean, readable code for learning OS development

### Architecture Diagram

```
┌─────────────────────────────────────────────────────┐
│                 User Space (Future)                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐         │
│  │  Shell   │  │  Grape   │  │   Apps   │         │
│  └──────────┘  └──────────┘  └──────────┘         │
└─────────────────────────────────────────────────────┘
                      ▲
                      │ IPC Messages
                      ▼
┌─────────────────────────────────────────────────────┐
│              Kernel Space (Current)                 │
│  ┌──────────────────────────────────────────────┐  │
│  │            Message Bus (IPC)                  │  │
│  └──────────────────────────────────────────────┘  │
│  ┌──────────┐  ┌──────────┐  ┌─────────────────┐  │
│  │   VFS    │  │ Terminal │  │  Other Services │  │
│  └──────────┘  └──────────┘  └─────────────────┘  │
│  ┌──────────────────────────────────────────────┐  │
│  │             Kernel Core                       │  │
│  │  ┌────────┐  ┌────────┐  ┌─────────────┐   │  │
│  │  │  IDT   │  │  GDT   │  │   Memory    │   │  │
│  │  └────────┘  └────────┘  └─────────────┘   │  │
│  └──────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────┐  │
│  │              Device Drivers                   │  │
│  │  ┌──────┐ ┌──────┐ ┌─────┐ ┌──────────────┐ │  │
│  │  │ FB   │ │ KBD  │ │Timer│ │    Serial    │ │  │
│  │  └──────┘ └──────┘ └─────┘ └──────────────┘ │  │
│  └──────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                      ▲
                      │ Hardware Access
                      ▼
┌─────────────────────────────────────────────────────┐
│                  Hardware Layer                     │
│  CPU │ RAM │ Framebuffer │ Keyboard │ Serial       │
└─────────────────────────────────────────────────────┘
```

---

## Boot Process

### 1. Firmware Stage (UEFI/BIOS)

```
Power On
  ↓
UEFI Firmware Initializes
  ↓
Reads ESP (EFI System Partition)
  ↓
Loads Limine Bootloader
```

### 2. Bootloader Stage (Limine)

**File**: `kernel/limine.cfg`

```properties
TIMEOUT 5
DEFAULT 0

:default
    KERNEL_PATH /kernel.elf
    SERIAL yes
    SERIAL_BAUDRATE 115200
```

**Limine Responsibilities:**
- Load kernel.elf into memory
- Set up paging (higher-half kernel)
- Provide memory map
- Initialize framebuffer (UEFI GOP or BIOS VGA)
- Pass boot information via Limine protocol

### 3. Kernel Entry Point

**File**: `kernel/src/main.rs`

```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Initialize GDT (Global Descriptor Table)
    arch::gdt::init();
    
    // 2. Initialize IDT (Interrupt Descriptor Table)
    interrupts::init();
    
    // 3. Initialize framebuffer console
    drivers::framebuffer::init();
    
    // 4. Parse Limine boot information
    boot::init();
    
    // 5. Initialize memory allocator
    allocator::init();
    
    // 6. Initialize device drivers
    drivers::init_all();
    
    // 7. Enable interrupts
    x86_64::instructions::interrupts::enable();
    
    // 8. Start main loop
    kernel_main();
}
```

### 4. Boot Sequence Diagram

```
UEFI/BIOS
    │
    ├─► Initialize Hardware
    │
    └─► Load Limine (from ESP)
            │
            ├─► Parse limine.cfg
            ├─► Load kernel.elf
            ├─► Set up HHDM (0xFFFF800000000000)
            ├─► Initialize GOP framebuffer
            │
            └─► Jump to kernel entry (_start)
                    │
                    ├─► [1] GDT Setup
                    ├─► [2] IDT Setup  
                    ├─► [3] Framebuffer Init
                    ├─► [4] Parse Limine Tags
                    ├─► [5] Heap Allocator Init
                    ├─► [6] Device Drivers Init
                    │       ├─► Serial (COM1)
                    │       ├─► Keyboard (PS/2)
                    │       ├─► Timer (PIT 100Hz)
                    │       └─► Framebuffer
                    ├─► [7] VFS Init (Unix-like tree)
                    ├─► [8] Enable Interrupts (sti)
                    │
                    └─► Main Loop
                            ├─► Process Keyboard Input
                            ├─► Blink Cursor (500ms)
                            └─► Idle (hlt)
```

---

## Kernel Architecture

### Source Tree

```
kernel/src/
├── main.rs              - Entry point, initialization
├── lib.rs               - Module declarations
├── arch/                - Architecture-specific code
│   └── x86_64/
│       ├── gdt.rs       - Global Descriptor Table
│       └── idt.rs       - Interrupt Descriptor Table
├── drivers/             - Hardware drivers
│   ├── framebuffer.rs   - Display driver (VGA/UEFI)
│   ├── keyboard.rs      - PS/2 keyboard driver
│   ├── timer.rs         - PIT timer (jiffies)
│   └── serial.rs        - COM1 serial port
├── ipc/                 - Inter-Process Communication
│   ├── bus.rs           - Message bus dispatcher
│   └── message.rs       - Message types
├── services/            - Kernel services
│   ├── vfs.rs           - Virtual Filesystem
│   └── terminal.rs      - Terminal service
├── grape/               - Text editor
│   └── mod.rs           - Grape editor implementation
├── shell/               - Command shell
│   └── mod.rs           - Shell command interpreter
├── interrupts.rs        - Interrupt handlers
├── boot.rs              - Limine protocol parsing
└── allocator.rs         - Heap memory allocator
```

### Module Dependencies

```
main
  ├─► arch::gdt
  ├─► interrupts
  │     └─► arch::idt
  ├─► boot (Limine)
  ├─► allocator
  ├─► drivers
  │     ├─► framebuffer
  │     ├─► keyboard
  │     ├─► timer
  │     └─► serial
  ├─► services
  │     ├─► vfs
  │     └─► terminal
  ├─► ipc
  │     ├─► bus
  │     └─► message
  ├─► grape (editor)
  └─► shell
```

---

## Memory Management

### Address Space Layout

```
0xFFFFFFFF_FFFFFFFF  ┐
                     │  Kernel Stack
                     │
0xFFFFFFFF_80000000  ┤  Kernel Code & Data (.text, .rodata, .data, .bss)
                     │
                     │  Heap (dynamic allocation, up to 32 MB)
                     │
0xFFFF8000_00000000  ┤  HHDM Base (Higher Half Direct Map)
                     │  Physical memory mapped here
                     │
0x00000000_00000000  ┴  (User space - future)
```

### Limine Memory Map

**File**: `kernel/src/boot.rs`

```rust
pub fn memory_map() -> Option<&'static [&'static limine::MemoryMapEntry]> {
    // Parse Limine memory map entries
    // Returns usable memory regions
}
```

**Memory Regions:**
- Usable: Available for OS use
- Reserved: Firmware/BIOS reserved
- ACPI Reclaimable: Can be reclaimed after ACPI parsing
- ACPI NVS: ACPI Non-Volatile Storage
- Bad Memory: Defective regions

### Heap Allocator

**File**: `kernel/src/allocator.rs`

```rust
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init() {
    // Find largest usable memory region from Limine
    // Initialize heap (up to 32 MB)
    unsafe {
        ALLOCATOR.lock().init(heap_start, heap_size);
    }
}
```

**Heap Size**: Currently up to 32 MB
**Algorithm**: Linked list allocator (simple but functional)

### Memory Allocation Example

```rust
use alloc::vec::Vec;
use alloc::string::String;

// Vector allocation
let mut vec = Vec::new();
vec.push(42);

// String allocation  
let s = String::from("Hello, ospabOS!");

// Box allocation
let b = Box::new(100);
```

---

## Interrupt Handling

### Interrupt Descriptor Table (IDT)

**File**: `kernel/src/interrupts.rs`

```rust
use x86_64::structures::idt::InterruptDescriptorTable;

static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    
    // CPU Exceptions
    idt.divide_error.set_handler_fn(divide_error_handler);
    idt.page_fault.set_handler_fn(page_fault_handler);
    idt.double_fault.set_handler_fn(double_fault_handler)
        .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
    
    // Hardware Interrupts
    idt[InterruptIndex::Timer.as_usize()]
        .set_handler_fn(timer_interrupt_handler);
    idt[InterruptIndex::Keyboard.as_usize()]
        .set_handler_fn(keyboard_interrupt_handler);
    
    idt
});
```

### Programmable Interrupt Controller (PIC)

**Configuration:**
- Master PIC: IRQ 0-7 (offset 32)
- Slave PIC: IRQ 8-15 (offset 40)

```
IRQ 0  - Timer (PIT)
IRQ 1  - Keyboard
IRQ 2  - Cascade (slave PIC)
IRQ 3  - COM2
IRQ 4  - COM1
IRQ 5  - LPT2
IRQ 6  - Floppy
IRQ 7  - LPT1
IRQ 8  - RTC
IRQ 14 - Primary ATA
IRQ 15 - Secondary ATA
```

### Timer Interrupt Handler

```rust
extern "x86-interrupt" fn timer_interrupt_handler(_frame: InterruptStackFrame) {
    // Increment jiffies (100 Hz = 10ms per tick)
    drivers::timer::tick();
    
    // Send EOI to PIC
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}
```

### Keyboard Interrupt Handler

```rust
extern "x86-interrupt" fn keyboard_interrupt_handler(_frame: InterruptStackFrame) {
    // Read scancode from port 0x60
    let scancode = unsafe {
        Port::<u8>::new(0x60).read()
    };
    
    // Queue scancode (lock-free)
    drivers::keyboard::queue_scancode(scancode);
    
    // Send EOI
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
```

---

## Device Drivers

### Framebuffer Driver

**File**: `kernel/src/drivers/framebuffer.rs`

**Features:**
- UEFI GOP (Graphics Output Protocol) support
- BIOS VGA compatibility
- 8x16 character font
- Rectangular blinking cursor (Linux-style)
- RGB/BGR auto-detection
- Scrolling support

**Resolution**: Determined by firmware (typically 1280x800 or 1024x768)

**Character Grid**: 
- Width: screen_width / 8
- Height: screen_height / 16

**Cursor:**
```rust
pub fn draw_cursor_at(row: usize, col: usize, visible: bool) {
    // Draw 8x16 block at (col * 8, row * 16)
    // Color: white if visible, black if hidden
}

// Blinks every 50 ticks (500ms)
pub fn toggle_cursor() {
    cursor_visible = !cursor_visible;
    draw_cursor(cursor_visible);
}
```

### Keyboard Driver

**File**: `kernel/src/drivers/keyboard.rs`

**Features:**
- PS/2 keyboard protocol
- Scancode Set 1 decoding (via `pc-keyboard` crate)
- Lock-free ring buffer for ISR
- Command history (5 entries)
- Arrow key navigation (left/right/up/down)
- Ctrl key combinations

**Architecture:**

```
Keyboard Hardware
       │
       ▼
  IRQ 1 Interrupt
       │
       ▼
keyboard_interrupt_handler (ISR)
       │
       ├─► Read port 0x60
       └─► queue_scancode() [Lock-free]
               │
               ▼
       Atomic Ring Buffer
               │
               ▼
   Main Loop: poll_input()
               │
               ├─► Dequeue scancode
               ├─► Decode with pc-keyboard
               └─► Handle key event
                       │
                       ├─► Unicode char → handle_char()
                       └─► Raw key → handle_arrow_*()
```

**EditorKey Enum:**

```rust
pub enum EditorKey {
    Char(char),
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    PageUp,
    PageDown,
    Home,
    End,
    Delete,
}

pub fn read_editor_key_blocking() -> Option<EditorKey> {
    // Returns both characters and navigation keys
    // Used by Grape editor
}
```

### Timer Driver

**File**: `kernel/src/drivers/timer.rs`

**Programmable Interval Timer (PIT):**
- Base frequency: 1.193182 MHz
- Target frequency: 100 Hz (10ms per tick)
- Divisor: 11932

**Jiffies Counter:**

```rust
static JIFFIES: AtomicU64 = AtomicU64::new(0);

pub fn tick() {
    JIFFIES.fetch_add(1, Ordering::Relaxed);
}

pub fn get_jiffies() -> u64 {
    JIFFIES.load(Ordering::Relaxed)
}

pub fn get_uptime_ms() -> u64 {
    get_jiffies() * 10  // 10ms per tick
}
```

### Serial Driver

**File**: `kernel/src/drivers/serial.rs`

**COM1 Configuration:**
- Port: 0x3F8
- Baud rate: 115200
- Data bits: 8
- Parity: None
- Stop bits: 1

**Usage:**

```rust
serial_print(b"Hello from serial!\r\n");
```

**Output:** Real-time kernel messages for debugging

---

## Filesystem (VFS)

**File**: `kernel/src/services/vfs.rs`

### Unix-like Hierarchy

```
/
├── bin/            System binaries (descriptive text, not executables yet)
│   ├── ls
│   ├── cat
│   └── grape
├── etc/            Configuration files
│   ├── hostname    → "ospabOS\n"
│   └── os-release  → VERSION="0.38"
├── home/
│   └── user/       User home directory
├── dev/            Device files (special)
│   ├── null        Device ID: 0
│   ├── zero        Device ID: 1
│   ├── keyboard    Device ID: 2
│   ├── framebuffer Device ID: 3
│   └── serial      Device ID: 4
├── tmp/            Temporary files (in-memory)
├── usr/
│   └── bin/        User programs
└── var/
    └── log/        System logs
```

### VNode Structure

```rust
pub enum FileType {
    Regular,    // Regular file
    Directory,  // Directory
    Device,     // Device file
    Link,       // Symbolic link (NYI)
}

pub struct VNode {
    pub name: String,
    pub file_type: FileType,
    pub size: usize,
    pub data: Option<Vec<u8>>,              // For regular files
    pub children: Option<BTreeMap<String, VNode>>,  // For directories
    pub device_id: Option<usize>,           // For device files
}
```

### Path Resolution

```rust
fn resolve_path(&self, path: &str) -> Option<VNode> {
    // Supports:
    // - Absolute paths: /etc/hostname
    // - Relative paths: etc/hostname (from cwd)
    // - Parent directory: ..
    // - Current directory: .
}
```

### Operations

```rust
pub enum FSRequest {
    ListDir { path: String },
    ReadFile { path: String },
    WriteFile { path: String, data: Vec<u8> },
    CreateDir { path: String },
    Delete { path: String },
    ChangeDir { path: String },
    GetCwd,
}

pub enum FSResponse {
    DirListing(Vec<String>),
    FileData(Vec<u8>),
    Cwd(String),
    Success,
    Error(String),
}
```

### Current Limitations

- **Read-only**: Cannot write files (no persistent storage yet)
- **In-memory only**: Lost on reboot
- **No permissions**: All files accessible
- **No timestamps**: Creation/modification time not tracked

---

## IPC & Microkernel

**Files**: `kernel/src/ipc/`

### Message Bus Architecture

```
┌──────────┐          ┌──────────────┐          ┌──────────┐
│  Shell   │ ───────► │ Message Bus  │ ───────► │   VFS    │
└──────────┘          └──────────────┘          └──────────┘
     │                       │                        │
     │                       │                        │
     ▼                       ▼                        ▼
FSRequest::ReadFile    Dispatch to VFS      Read /etc/hostname
                                                     │
                                                     ▼
                                             FSResponse::FileData
```

### Message Types

```rust
pub enum Message {
    FSRequest(FSRequest),
    UIRequest(UIRequest),
    // More message types in future
}

pub enum FSRequest {
    ListDir { path: String },
    ReadFile { path: String },
    // ...
}

pub enum UIRequest {
    Print { text: String },
    SetCursor { x: usize, y: usize },
    // ...
}
```

### Service Registration

```rust
pub struct MessageBus {
    vfs_queue: VecDeque<FSRequest>,
    terminal_queue: VecDeque<UIRequest>,
}

impl MessageBus {
    pub fn send_fs_request(&mut self, req: FSRequest) -> FSResponse {
        // Currently synchronous, will be async in future
        vfs::process_request(req)
    }
}
```

### Future Architecture (v0.41+)

```
User Space:
  Shell Process ──┐
  Grape Process ──┼─► IPC Messages → Kernel
  App Processes ──┘

Kernel Space:
  Message Bus ──┐
                ├─► VFS Service (process_id=1)
                ├─► Terminal Service (process_id=2)
                └─► Other Services
```

---

## Shell & Text Editor

### Shell

**File**: `kernel/src/shell/mod.rs`

**Features:**
- Command parsing
- Directory navigation (cd, pwd)
- File operations (ls, cat)
- System info (version, uptime)
- History (5 commands)

**Prompt**: `[ospab]~> `

**Implementation:**

```rust
pub fn execute_command(cmd: &str) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() { return; }
    
    match parts[0] {
        "ls" => { /* List directory */ },
        "cat" => { /* Display file */ },
        "cd" => { /* Change directory */ },
        "grape" => { /* Open text editor */ },
        _ => framebuffer::print("Unknown command\n"),
    }
}
```

### Grape Text Editor

**File**: `kernel/src/grape/mod.rs`

**Features:**
- Load/save files (VFS)
- Multi-line editing
- Arrow key navigation
- Page Up/Down scrolling
- Home/End keys
- Backspace/Delete
- Modified flag tracking

**Keybindings:**

| Key | Function |
|-----|----------|
| Ctrl+G | Help |
| Ctrl+X | Save |
| Ctrl+C | Exit (with unsaved check) |
| Ctrl+W | Search (NYI) |
| Ctrl+K | Cut (NYI) |
| Ctrl+U | Paste (NYI) |
| Arrows | Navigate |
| PgUp/PgDn | Scroll 10 lines |
| Home/End | Line start/end |
| Delete | Delete forward |
| Backspace | Delete backward |

**Structure:**

```rust
pub struct GrapeEditor {
    filename: String,
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    scroll_offset: usize,
    modified: bool,
    message: Option<String>,
    max_rows: usize,
}

impl GrapeEditor {
    pub fn open(filename: &str) -> Result<(), String> {
        // 1. Load file from VFS
        // 2. Enter event loop
        // 3. Handle keyboard input
        // 4. Draw screen + cursor
        // 5. Save on Ctrl+X, exit on Ctrl+C
    }
}
```

---

## Development Roadmap

### v0.39 (Next)
- [ ] Implement search in Grape (Ctrl+W)
- [ ] Syntax highlighting for Rust/C
- [ ] More shell commands (mkdir, rm, cp, mv)

### v0.40
- [ ] FAT32 filesystem driver (read/write)
- [ ] AHCI disk driver (SATA)
- [ ] Persistent storage

### v0.41-v0.45
- [ ] Process management (fork/exec)
- [ ] User mode / kernel mode separation
- [ ] System calls (syscall instruction)
- [ ] Basic scheduler (round-robin)

### v0.46-v0.50
- [ ] E1000 network driver
- [ ] TCP/IP stack (smoltcp)
- [ ] HTTP client

### v1.0+
- [ ] SMP support (multi-core)
- [ ] Port Doom
- [ ] GUI framework (framebuffer-based)
- [ ] Package manager

---

## Performance Characteristics

### Boot Time
- QEMU: ~1-2 seconds
- Real hardware: ~3-5 seconds (depends on firmware)

### Memory Usage
- Kernel size: ~1.4 MB ISO
- Runtime heap: ~2-5 MB (depends on usage)
- Framebuffer: ~4 MB (1280x800x4 bytes)

### Interrupt Latency
- Timer: 10ms period (100 Hz)
- Keyboard: <1ms response

---

## Known Issues

1. **Read-only filesystem**: No persistent storage yet
2. **Single-core**: No SMP support
3. **No networking**: Network stack not implemented
4. **No sound**: Audio drivers not implemented
5. **USB storage**: Only keyboard supported, not mass storage

---

## References

- **Rust**: https://rust-lang.org/
- **Limine**: https://github.com/limine-bootloader/limine
- **OSDev Wiki**: https://wiki.osdev.org/
- **x86_64 crate**: https://docs.rs/x86_64/
- **pc-keyboard**: https://docs.rs/pc-keyboard/

---

*For installation instructions, see [BARE_METAL_GUIDE.md](BARE_METAL_GUIDE.md)*
*For general information, see [README.md](README.md)*
