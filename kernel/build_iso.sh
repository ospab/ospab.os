#!/usr/bin/env bash

# Robust ISO builder for ospabOS
# - builds release kernel if missing
# - copies Limine artifacts from kernel/tools/limine if available
# - places kernel at /kernel.elf inside ISO
# - creates minimal limine.cfg if missing

set -euo pipefail

ISO_ROOT="iso_root"
EFI_DIR="$ISO_ROOT/EFI/BOOT"
# default release kernel path (can be overridden with KERNEL_BINARY env)
: ${KERNEL_BINARY:=""}
# If KERNEL_BINARY not provided, try to discover common artifact locations
if [ -z "$KERNEL_BINARY" ]; then
  # prefer explicit kernel.elf in any target dir
  candidate=$(find target -type f -path "*/release/kernel.elf" -print -quit 2>/dev/null || true)
  if [ -z "$candidate" ]; then
    # look for a built binary named ospab-os in any target triple release dir
    candidate=$(find target -type f -name "ospab-os" -path "*/release/*" -print -quit 2>/dev/null || true)
  fi
  if [ -z "$candidate" ]; then
    # fallback to target/release/ospab-os
    if [ -f "target/release/ospab-os" ]; then
      candidate="target/release/ospab-os"
    fi
  fi
  if [ -n "$candidate" ]; then
    KERNEL_BINARY="$candidate"
  else
    KERNEL_BINARY="target/x86_64-ospab_os/release/kernel.elf"
  fi
fi

echo "[build_iso] Resolved kernel binary: $KERNEL_BINARY"
LIMINE_CFG="limine.cfg"
OUTPUT_ISO="ospab-os.iso"

mkdir -p "$ISO_ROOT"
mkdir -p "$EFI_DIR"

echo "[build_iso] Using kernel binary: $KERNEL_BINARY"

# If kernel not found, try to build release
if [ ! -f "$KERNEL_BINARY" ]; then
  echo "[build_iso] Kernel not found, building release..."
  cargo +nightly build --release
  if [ ! -f "$KERNEL_BINARY" ]; then
    echo "[build_iso] ERROR: kernel binary still missing after build: $KERNEL_BINARY" >&2
    exit 1
  fi
fi

# Copy kernel into ISO root as /kernel.elf (matches limine.cfg PATH)
cp -v "$KERNEL_BINARY" "$ISO_ROOT/kernel.elf"

# Ensure limine.cfg exists in iso root; if not, create a minimal one
if [ -f "$LIMINE_CFG" ]; then
  cp -v "$LIMINE_CFG" "$ISO_ROOT/limine.cfg"
else
  cat > "$ISO_ROOT/limine.cfg" <<'EOF'
PROTOCOL limine

TIMEOUT 5

:/ospabOS
    PATH /kernel.elf
    TIMEOUT 0
    SERIAL yes
    SERIAL_BAUDRATE 115200
    COMMENT "ospabOS"
EOF
  echo "[build_iso] Wrote minimal $ISO_ROOT/limine.cfg"
fi

# Try to find Limine artifacts under kernel/tools/limine
LIMINE_TOOLS_DIR="$(dirname "$0")/tools/limine"
if [ -d "$LIMINE_TOOLS_DIR/bin" ]; then
  echo "[build_iso] Copying Limine binaries from $LIMINE_TOOLS_DIR/bin"
  cp -v "$LIMINE_TOOLS_DIR/bin/limine-bios-cd.bin" "$ISO_ROOT/" 2>/dev/null || true
  cp -v "$LIMINE_TOOLS_DIR/bin/limine-bios-hdd.bin" "$ISO_ROOT/" 2>/dev/null || true
  cp -v "$LIMINE_TOOLS_DIR/bin/limine-bios.sys" "$ISO_ROOT/" 2>/dev/null || true
fi

# Try to find a UEFI BOOTX64.EFI produced by Limine (several possible locations)
UEFI_CANDIDATES=(
  "$LIMINE_TOOLS_DIR/common-uefi-x86-64/BOOTX64.EFI"
  "$LIMINE_TOOLS_DIR/bin/BOOTX64.EFI"
  "$LIMINE_TOOLS_DIR/BUILDDIR/common-uefi-x86-64/BOOTX64.EFI"
  "$LIMINE_TOOLS_DIR/BOOTX64.EFI"
)
for f in "${UEFI_CANDIDATES[@]}"; do
  if [ -f "$f" ]; then
    echo "[build_iso] Found UEFI image: $f -> $EFI_DIR/BOOTX64.EFI"
    cp -v "$f" "$EFI_DIR/BOOTX64.EFI"
    break
  fi
done

# Look for eltorito EFI image (limine-eltorito-efi.bin)
if [ -f "$LIMINE_TOOLS_DIR/bin/limine-eltorito-efi.bin" ]; then
  cp -v "$LIMINE_TOOLS_DIR/bin/limine-eltorito-efi.bin" "$ISO_ROOT/limine-eltorito-efi.bin"
fi

echo "[build_iso] Creating ISO with xorriso..."

# Build xorriso command conditionally depending on whether we have an eltorito EFI image
if [ -f "$ISO_ROOT/limine-eltorito-efi.bin" ]; then
  echo "[build_iso] Found limine-eltorito-efi.bin; creating hybrid BIOS+UEFI ISO"
  xorriso -as mkisofs \
    -o "$OUTPUT_ISO" \
    -V "ospabOS" \
    -J -R -l \
    -b limine-bios-cd.bin \
    -no-emul-boot \
    -boot-load-size 4 \
    -boot-info-table \
    --efi-boot limine-eltorito-efi.bin \
    -efi-boot-part --efi-boot-image --protective-msdos-label \
    "$ISO_ROOT"
else
  echo "[build_iso] No limine-eltorito-efi.bin found; creating BIOS-only ISO"
  xorriso -as mkisofs \
    -o "$OUTPUT_ISO" \
    -V "ospabOS" \
    -J -R -l \
    -b limine-bios-cd.bin \
    -no-emul-boot \
    -boot-load-size 4 \
    -boot-info-table \
    "$ISO_ROOT"
fi

# Install Limine (bios) into the ISO if limine-install available
if command -v limine-install >/dev/null 2>&1; then
  echo "[build_iso] Running limine-install on $OUTPUT_ISO"
  limine-install "$OUTPUT_ISO"
else
  echo "[build_iso] limine-install not found in PATH; skip installation step"
fi

echo "[build_iso] ISO built successfully: $OUTPUT_ISO"

