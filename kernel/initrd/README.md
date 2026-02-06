# ospabOS Kernel - README

## About
ospabOS is a microkernel operating system written in Rust for x86_64 architecture.

## Architecture
- Message-passing IPC with central Message Bus
- Terminal Service (wraps framebuffer and keyboard I/O)
- VFS Service (initrd-based filesystem)
- Shell command interpreter

## Building
```bash
wsl bash build_with_alloc.sh
```

## Running
```bash
qemu-system-x86_64 -cdrom ospab-os-xx.iso -m 128M -serial stdio
```

## License
MIT License - See LICENSE file for details
