# ospabOS

A hobby operating system written in Rust, booted via Limine bootloader.

## Features

- **Architecture**: x86_64 bare metal
- **Bootloader**: Limine v10.6.3 (BIOS mode)
- **Language**: Rust (nightly)
- **Framebuffer**: Direct VGA-style console with 12x12 bitmap font
- **Interrupts**: Full IDT setup with PIC management
- **Memory**: GDT and basic memory map access
- **Keyboard**: PS/2 keyboard driver with command shell

## Current Status

✅ **Stable Build** - The kernel boots successfully and runs stably with keyboard processing disabled.

### Working:
- ✅ GDT and IDT initialization
- ✅ PIC configuration (IRQ0 timer, IRQ1 keyboard)
- ✅ Framebuffer console with white on black display
- ✅ Memory map access via Limine protocol
- ✅ Keyboard interrupt handling (scancodes read and discarded)
- ✅ Stable idle loop (no crashes or reboots)

### Temporarily Disabled:
- ⚠️ Keyboard input processing (causes triple fault)
- ⚠️ Command shell (depends on keyboard)

### Known Issues:
- ❌ Keyboard scancode processing causes triple fault in VMware and QEMU
- ❌ Calling `framebuffer::print` from keyboard handler or main loop triggers crash
- ⚠️ No memory allocator yet
- ⚠️ Clear screen disabled (too slow with write_volatile)

## Building

### Prerequisites

- Rust nightly toolchain
- `rust-src` component
- WSL with `xorriso` (for ISO creation on Windows)
- QEMU (for testing)

### Build Commands

```powershell
cd kernel
cargo +nightly build --release

# Create ISO (Windows with WSL)
wsl bash -c "cp target/x86_64-ospab/release/ospab-os iso_root/kernel.elf && \
  cd iso_root && \
  xorriso -as mkisofs -b limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --protective-msdos-label . -o ../ospab.iso"
```

### Running

```powershell
# QEMU (recommended)
qemu-system-x86_64 -cdrom kernel/ospab.iso -m 128M -serial mon:stdio

# VMware (currently broken)
# Load kernel/ospab.iso in VMware
```

## Project Structure

```
ospab.os/
├── kernel/                    # Main kernel crate
│   ├── src/
│   │   ├── main.rs           # Kernel entry point
│   │   ├── boot.rs           # Limine protocol interface
│   │   ├── gdt.rs            # Global Descriptor Table
│   │   ├── interrupts.rs     # IDT and exception handlers
│   │   └── drivers/
│   │       ├── framebuffer.rs # VGA framebuffer console
│   │       └── keyboard.rs    # PS/2 keyboard driver
│   ├── iso_root/             # ISO filesystem root
│   │   ├── kernel.elf        # Compiled kernel (copied during build)
│   │   ├── limine.conf       # Bootloader configuration
│   │   └── limine-*.bin      # Bootloader binaries
│   ├── Cargo.toml            # Rust project configuration
│   └── x86_64-ospab.json     # Custom target specification
└── README.md                 # This file
```

## Architecture Details

### Boot Process

1. **Limine bootloader** loads kernel in BIOS mode
2. **Kernel entry** (`_start`) disables interrupts
3. **HHDM offset** retrieved for higher-half direct mapping
4. **GDT initialization** sets up segmentation
5. **IDT setup** configures exception and interrupt handlers
6. **PIC configuration** unmasks IRQ0 (timer) and IRQ1 (keyboard)
7. **Framebuffer init** from Limine framebuffer tag
8. **Keyboard init** creates PS/2 decoder instance
9. **Interrupts enabled** - system enters main loop

### Interrupt Handling

- **ISR Strategy**: Keyboard ISR only queues scancodes to ring buffer
- **Main Loop**: Processes queued scancodes outside interrupt context
- **Reason**: `framebuffer::print` uses `write_volatile` which is slow and unsafe in ISR

### Memory Management

- **No allocator**: All data structures are static
- **HHDM**: Higher-half direct mapping at `0xFFFF800000000000`
- **Framebuffer**: Memory-mapped I/O via Limine framebuffer tag
- **Static mut**: Used extensively (no spin locks due to early boot issues)

## Debugging

### Triple Fault Issues

The kernel experiences triple faults when processing keyboard scancodes. Current investigation:

**Root Cause**: Calling `framebuffer::print` (which uses `write_volatile`) appears to trigger the fault, even when called from main loop outside ISR context.

**Attempted Fixes**:
1. ✅ **Ring buffer approach** - ISR only queues scancodes, main loop processes them
2. ✅ **Fixed PIC EOI** - Use IRQ numbers 0/1, not interrupt vector 32/33
3. ✅ **Removed spin locks** - Replaced `spin::Once` with `static mut + Option`
4. ✅ **Bounds checking** - Added strict framebuffer boundary checks
5. ✅ **Safety guards** - Check `is_initialized()` before all framebuffer operations

**Current Workaround**: Keyboard processing completely disabled. System boots and runs stably.

### Serial Debugging

Serial output on COM1 (0x3F8) logs all boot stages:

```
[BOOT] Checking Limine protocol... OK
[INIT] Initializing GDT... OK
[INIT] Initializing IDT and PICs... OK
[BOOT] Checking framebuffer... OK
[INIT] Initializing keyboard driver... OK
[READY] System initialized
```

### Error Handling

All exception handlers:
- Print error message to serial port
- Halt forever (no reboot to preserve state)

## Development Timeline

### Issues Fixed

1. **GDT crash** - Changed from spin::Once to static mut
2. **IDT crash** - Same as GDT
3. **Framebuffer pixel writes** - Added write_volatile everywhere
4. **Memory map crash** - Fixed iterator implementation
5. **Small font** - Scaled 8x8 to 12x12 with nearest-neighbor
6. **Blue background** - Changed to black (0x000000)
7. **Keyboard not working** - Fixed PIC masks (0b11111100)
8. **Keyboard hang** - Removed spin locks, used static mut
9. **Triple fault from keyboard** - Moved processing to main loop

### Current Investigation

Working on resolving VMware triple fault. The kernel is stable in QEMU but crashes in VMware, suggesting:
- More strict memory access checking in VMware
- Possible timing issues with hardware emulation
- Need for memory barriers or additional synchronization

## License

This is a hobby/educational project. Feel free to use the code for learning purposes.

## Author

ospab

## References

- [Limine Boot Protocol](https://github.com/limine-bootloader/limine)
- [OSDev Wiki](https://wiki.osdev.org/)
- [Writing an OS in Rust](https://os.phil-opp.com/)
- [x86_64 crate](https://docs.rs/x86_64/)
