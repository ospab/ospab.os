#!/usr/bin/env bash
set -euo pipefail

# rebuild.sh - Full clean + release build + ISO assembly for ospabOS
# - Removes cargo target and iso_root
# - Builds release kernel (stable configuration)
# - Copies kernel to iso_root/kernel.elf (lowercase)
# - Writes a minimal limine.conf with PROTOCOL limine and KERNEL_PATH boot:///kernel.elf
# - Uses the xorriso binary at D:/Toolz/xorriso/xorriso.exe when available

SCRIPT_DIR="/mnt/d/ospab-projects/ospab.os/kernel"
ISO_ROOT="$SCRIPT_DIR/iso_root"
# Force final ISO destination to Windows-mounted workspace path as requested
OUTPUT_ISO="/mnt/d/ospab-projects/ospab.os/kernel/ospab-os.iso"
XORRISO_WIN='D:/Toolz/xorriso/xorriso.exe'

echo "[rebuild] Cleaning previous build artifacts..."
# Try to remove files; if they're on a mounted Windows filesystem you may need sudo
rm -rf "$SCRIPT_DIR/target" "$ISO_ROOT" "$OUTPUT_ISO" || {
  echo "[rebuild] Warning: unable to remove some artifacts directly; trying with sudo..."
  sudo rm -rf "$SCRIPT_DIR/target" "$ISO_ROOT" "$OUTPUT_ISO" || true
}

echo "[rebuild] Building release kernel (this may take a few moments)..."
# Use nightly as required by the project
cargo +nightly build -Z build-std=core --target x86_64-unknown-none --release

# Discover built artifact
echo "[rebuild] Locating built kernel ELF..."
KERNEL_BIN=""
# Prefer named kernel.elf in release dirs
KERNEL_BIN=$(find "$SCRIPT_DIR/target" -type f -iname 'kernel.elf' -path '*/release/*' -print -quit || true)
if [ -z "$KERNEL_BIN" ]; then
  # fallback to any release artifact named ospab-os
  KERNEL_BIN=$(find "$SCRIPT_DIR/target" -type f -name 'ospab-os' -path '*/release/*' -print -quit || true)
fi
if [ -z "$KERNEL_BIN" ]; then
  echo "ERROR: Could not find built kernel in target/*/release. Aborting." >&2
  exit 1
fi

echo "[rebuild] Found kernel: $KERNEL_BIN"

mkdir -p "$ISO_ROOT"

# Copy kernel to iso_root/kernel.elf (lowercase) as required by Limine config
echo "[rebuild] Copying kernel into $ISO_ROOT/kernel.elf"
cp -v "$KERNEL_BIN" "$ISO_ROOT/kernel.elf"

# Write limine.conf with PROTOCOL=limine and KERNEL_PATH boot:///kernel.elf
cat > "$ISO_ROOT/limine.conf" <<'EOF'
PROTOCOL limine

KERNEL_PATH: boot:///kernel.elf

timeout: 3
serial: yes
serial_baudrate: 115200

/ospabOS
    path: boot:///kernel.elf
    comment: ospabOS kernel
    protocol: limine
EOF

# Copy limine bios images if present in tools
if [ -d "$SCRIPT_DIR/tools/limine/bin" ]; then
  echo "[rebuild] Copying Limine bios files to iso_root"
  cp -v "$SCRIPT_DIR/tools/limine/bin"/limine-bios-* "$ISO_ROOT/" 2>/dev/null || true
fi

# Resolve xorriso location; prefer native WSL xorriso when available
XORRISO=""
if command -v xorriso >/dev/null 2>&1; then
  XORRISO="$(command -v xorriso)"
elif [ -x "$XORRISO_WIN" ]; then
  XORRISO="$XORRISO_WIN"
elif [ -x "/mnt/d/Toolz/xorriso/xorriso.exe" ]; then
  XORRISO="/mnt/d/Toolz/xorriso/xorriso.exe"
else
  echo "ERROR: xorriso not found in PATH or at $XORRISO_WIN. Install xorriso or set PATH." >&2
  exit 1
fi

# Build ISO directly into the requested output path
OUTPUT_ISO_ABS="$(realpath "$OUTPUT_ISO")"

# If target exists under /mnt, remove it first (try sudo if necessary)
if [[ "$OUTPUT_ISO_ABS" == /mnt/* ]]; then
  if [ -f "$OUTPUT_ISO_ABS" ]; then
    echo "[rebuild] Removing existing ISO at $OUTPUT_ISO_ABS"
    rm -f "$OUTPUT_ISO_ABS" || sudo rm -f "$OUTPUT_ISO_ABS" || true
  fi
fi

echo "[rebuild] Creating ISO (using $XORRISO) -> $OUTPUT_ISO_ABS"
# If the xorriso binary is a Windows executable (.exe) run with Windows paths (use wslpath)
if [[ "$XORRISO" == *.exe ]]; then
  ISO_ROOT_WIN="$(wslpath -w "$ISO_ROOT")"
  OUTPUT_WIN="$(wslpath -w "$OUTPUT_ISO_ABS")"
  echo "[rebuild] Detected Windows xorriso binary, using Windows paths: $ISO_ROOT_WIN -> $OUTPUT_WIN"
  "$XORRISO" -as mkisofs -o "$OUTPUT_WIN" -V "ospabOS" -J -R -l \
    -b limine-bios-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table \
    "${ISO_ROOT_WIN}"
else
  "$XORRISO" -as mkisofs -o "$OUTPUT_ISO_ABS" -V "ospabOS" -J -R -l \
    -b limine-bios-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table \
    "$ISO_ROOT"
fi

# Try to run limine-install if available
if command -v limine-install >/dev/null 2>&1; then
  echo "[rebuild] Running limine-install on $OUTPUT_ISO_ABS"
  sudo limine-install "$OUTPUT_ISO_ABS"
else
  echo "[rebuild] limine-install not found in PATH; you should run it on the generated ISO to write El-Torito entries"
fi

echo "[rebuild] Done. Generated ISO: $OUTPUT_ISO_ABS"
exit 0
