Place a UEFI Limine binary named BOOTX64.EFI here to enable UEFI boot.

To build the EFI binary from the Limine sources in `kernel/tools/limine` (if needed):
1. Open a POSIX shell with required build tools (make, gcc/clang, mingw-w64 for PE/COFF if on Windows).
2. From `kernel/tools/limine`, run `make bin/BOOTX64.EFI` or the appropriate target that builds `common-uefi-x86-64/BOOTX64.EFI`.
3. Copy the produced `BOOTX64.EFI` into this folder and rebuild the ISO.

If you want, I can attempt to build the UEFI image here (requires a working make/toolchain)."}]{