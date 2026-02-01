#!/bin/bash
# Build script for ospabOS with proper alloc support

set -e

KERNEL_DIR="/mnt/d/ospab-projects/ospab.os/kernel"
ISOS_DIR="$KERNEL_DIR/isos"

# Find next ISO number
LAST_NUM=$(ls -1 "$ISOS_DIR"/ospab-os-*.iso 2>/dev/null | sed 's/.*ospab-os-\([0-9]*\)\.iso/\1/' | sort -n | tail -1)
if [ -z "$LAST_NUM" ]; then
    NEXT_NUM=1
else
    NEXT_NUM=$((LAST_NUM + 1))
fi
ISO_NAME="ospab-os-${NEXT_NUM}.iso"

echo "Building ospabOS kernel..."
echo "Output: $ISO_NAME"
cargo +nightly build --release -Z build-std=core,alloc --target x86_64-ospab.json

echo "Build successful!"
echo "Creating ISO #$NEXT_NUM..."

cd /tmp
rm -rf iso_root
mkdir -p iso_root/boot/limine iso_root/limine iso_root/EFI/BOOT

# Copy kernel
cp "$KERNEL_DIR/target/x86_64-ospab/release/ospab-os" iso_root/kernel

# Copy Limine config to ALL possible locations
cp "$KERNEL_DIR/iso_root/limine.conf" iso_root/limine.conf
cp "$KERNEL_DIR/iso_root/limine.conf" iso_root/limine/limine.conf  
cp "$KERNEL_DIR/iso_root/limine.conf" iso_root/boot/limine.conf
cp "$KERNEL_DIR/iso_root/limine.conf" iso_root/boot/limine/limine.conf
cp "$KERNEL_DIR/iso_root/limine.conf" iso_root/EFI/BOOT/limine.conf

# Copy Limine boot files
cp "$KERNEL_DIR/tools/limine/bin/limine-bios.sys" iso_root/
cp "$KERNEL_DIR/tools/limine/bin/limine-bios.sys" iso_root/boot/
cp "$KERNEL_DIR/iso_root/limine-bios-cd.bin" iso_root/
cp "$KERNEL_DIR/tools/limine/bin/BOOTX64.EFI" iso_root/EFI/BOOT/

# Create ISO with Rock Ridge extensions for proper filenames
xorriso -as mkisofs \
    -R \
    -b limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --efi-boot EFI/BOOT/BOOTX64.EFI \
    --efi-boot-part --efi-boot-image --protective-msdos-label \
    iso_root -o "$ISOS_DIR/$ISO_NAME"

"$KERNEL_DIR/tools/limine/bin/limine" bios-install "$ISOS_DIR/$ISO_NAME"

ls -lh "$ISOS_DIR/$ISO_NAME"
echo ""
echo "=== ISO created: $ISOS_DIR/$ISO_NAME ==="
