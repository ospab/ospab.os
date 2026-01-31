#!/bin/bash

# Exit on error
set -e

# Define paths
ISO_ROOT="iso_root"
BOOT_DIR="$ISO_ROOT/boot"
EFI_DIR="$ISO_ROOT/EFI/BOOT"
KERNEL_BINARY="target/x86_64-ospab_os/debug/kernel.elf"
LIMINE_CFG="limine.conf"
OUTPUT_ISO="ospab-os.iso"

# Create directory structure
mkdir -p $BOOT_DIR
mkdir -p $EFI_DIR

# Copy kernel and configuration files
cp $KERNEL_BINARY $BOOT_DIR/kernel.elf
cp $LIMINE_CFG $ISO_ROOT/limine.cfg

# Build the ISO
xorriso -as mkisofs \
  -b limine-cd.bin \
  -no-emul-boot \
  -boot-load-size 4 \
  -boot-info-table \
  --efi-boot limine-eltorito-efi.bin \
  -efi-boot-part --efi-boot-image --protective-msdos-label \
  $ISO_ROOT -o $OUTPUT_ISO

# Install Limine bootloader
limine-install $OUTPUT_ISO

# Output success message
echo "ISO built successfully: $OUTPUT_ISO"