#!/bin/bash
set -e

# Пути
KERNEL_DIR="/mnt/d/ospab-projects/ospab.os/kernel"
ISOS_DIR="$KERNEL_DIR/isos"
LIMINE_BIN_DIR="$KERNEL_DIR/tools/limine/bin"
ISO_LIMINE_ROOT="$KERNEL_DIR/iso_root"
ESP_IMG="/tmp/iso_root/efiboot.img"

# 1. Авто-инкремент номера ISO
mkdir -p "$ISOS_DIR"
LAST_NUM=$(ls -1 "$ISOS_DIR"/ospab-os-*.iso 2>/dev/null | sed 's/.*ospab-os-\([0-9]*\)\.iso/\1/' | sort -n | tail -1)
if [ -z "$LAST_NUM" ]; then
    NEXT_NUM=1
else
    NEXT_NUM=$((LAST_NUM + 1))
fi
ISO_NAME="ospab-os-${NEXT_NUM}.iso"
ISO_PATH="$ISOS_DIR/$ISO_NAME"

echo "--- Building Kernel (ISO #$NEXT_NUM) ---"
cd "$KERNEL_DIR"
cargo +nightly build --release -Z build-std=core,alloc --target x86_64-ospab.json

echo "--- Building User Shell ---"
USER_SHELL_DIR="/mnt/d/ospab-projects/ospab.os/user/shell"
USER_SHELL_TARGET="$KERNEL_DIR/x86_64-ospab.json"
if [ -d "$USER_SHELL_DIR" ]; then
    cd "$USER_SHELL_DIR"
    cargo +nightly build --release -Z build-std=core --target "$USER_SHELL_TARGET"
    mkdir -p "$KERNEL_DIR/initrd/bin"
    cp "$USER_SHELL_DIR/target/x86_64-ospab/release/ospabshell" "$KERNEL_DIR/initrd/bin/ospabshell"
    cd "$KERNEL_DIR"
else
    echo "WARN: user shell not found at $USER_SHELL_DIR"
fi

echo "--- Preparing ISO Root ---"
rm -rf /tmp/iso_root
# ВАЖНО: Создаем именно ту структуру, которую ищет Limine на твоих скринах
mkdir -p /tmp/iso_root/boot/limine
mkdir -p /tmp/iso_root/EFI/BOOT

# КОПИРУЕМ ЯДРО: кладем его в /boot/KERNEL, чтобы путь boot():/boot/KERNEL заработал
cp "$KERNEL_DIR/target/x86_64-ospab/release/ospab-os" /tmp/iso_root/boot/KERNEL

# Подготавливаем limine.conf с модулями initrd
BASE_CONF="$ISO_LIMINE_ROOT/limine.conf"
TMP_CONF="/tmp/iso_root/limine.conf"
# Копируем только если файл отсутствует или отличается (избегаем копирования файла на самого себя)
if [ ! -e "$TMP_CONF" ] || ! cmp -s "$BASE_CONF" "$TMP_CONF"; then
    cp "$BASE_CONF" "$TMP_CONF"
fi

# Копируем загрузчики
cp "$ISO_LIMINE_ROOT/limine-bios-cd.bin" /tmp/iso_root/boot/limine/
cp "$LIMINE_BIN_DIR/limine-bios.sys" /tmp/iso_root/boot/limine/
cp "$LIMINE_BIN_DIR/BOOTX64.EFI" /tmp/iso_root/EFI/BOOT/

# Копируем файлы initrd
mkdir -p /tmp/iso_root/initrd
if [ -d "$KERNEL_DIR/initrd" ]; then
    cp -r "$KERNEL_DIR/initrd/"* /tmp/iso_root/initrd/
fi

# Create a tarball so VFS can restore paths like /bin/ospabshell
if [ -d "$KERNEL_DIR/initrd" ]; then
    tar --format=ustar -C "$KERNEL_DIR/initrd" -cf /tmp/iso_root/initrd/initrd.tar .
fi

# Добавляем все .sh/.bash файлы в initrd
REPO_DIR="$(dirname "$KERNEL_DIR")"
find "$REPO_DIR" -type f \( -name "*.sh" -o -name "*.bash" \) -print0 | while IFS= read -r -d '' f; do
    base_name="$(basename "$f")"
    cp "$f" "/tmp/iso_root/initrd/$base_name"
done

# Добавляем module_path для всех файлов initrd
for f in /tmp/iso_root/initrd/*; do
    if [ -f "$f" ]; then
        name="$(basename "$f")"
        echo "    module_path: boot():/initrd/$name" >> "$TMP_CONF"
    fi
done

# Копируем конфиг в стандартные места (избегаем копирования на самого себя)
if [ ! -e /tmp/iso_root/boot/limine/limine.conf ] || ! cmp -s "$TMP_CONF" /tmp/iso_root/boot/limine/limine.conf; then
    cp "$TMP_CONF" /tmp/iso_root/boot/limine/limine.conf
fi
# TMP_CONF уже указывает на /tmp/iso_root/limine.conf — дополнительное копирование не нужно

echo "--- Creating 64MB FAT32 ESP Image ---"
rm -f "$ESP_IMG"
dd if=/dev/zero of="$ESP_IMG" bs=1M count=64 >/dev/null 2>&1
mkfs.vfat -F 32 "$ESP_IMG"
mmd -i "$ESP_IMG" ::/EFI ::/EFI/BOOT
mcopy -i "$ESP_IMG" "$LIMINE_BIN_DIR/BOOTX64.EFI" ::/EFI/BOOT/BOOTX64.EFI
mcopy -i "$ESP_IMG" "$TMP_CONF" ::/EFI/BOOT/limine.conf
if [ -f "$KERNEL_DIR/target/x86_64-ospab/release/ospab-os" ]; then
    mmd -i "$ESP_IMG" ::/boot >/dev/null 2>&1 || true
    mcopy -i "$ESP_IMG" "$KERNEL_DIR/target/x86_64-ospab/release/ospab-os" ::/boot/KERNEL
else
    echo "ERROR: Kernel binary not found for ESP copy" >&2
    exit 1
fi

# Копируем initrd в ESP, чтобы Limine в UEFI видел модули
mmd -i "$ESP_IMG" ::/initrd >/dev/null 2>&1 || true
for f in /tmp/iso_root/initrd/*; do
    if [ -f "$f" ]; then
        name="$(basename "$f")"
        mcopy -i "$ESP_IMG" "$f" "::/initrd/$name" >/dev/null 2>&1 || true
    fi
done

echo "--- Creating Hybrid ISO via Xorriso ---"
xorriso -as mkisofs \
    -iso-level 3 -R -J \
    -b boot/limine/limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    -eltorito-alt-boot \
    -eltorito-platform efi \
    -e efiboot.img \
    -no-emul-boot \
    -isohybrid-gpt-basdat \
    -append_partition 2 0xef "$ESP_IMG" \
    -appended_part_as_gpt \
    -partition_cyl_align all \
    /tmp/iso_root -o "$ISO_PATH"

echo "--- Post-processing ---"
# Вшиваем конфиг прямо в образ (enroll-config), чтобы Limine точно знал, где ядро
if [ -x "$LIMINE_BIN_DIR/limine" ]; then
    "$LIMINE_BIN_DIR/limine" enroll-config "$ISO_PATH" "$TMP_CONF" 2>/dev/null || true
    "$LIMINE_BIN_DIR/limine" bios-install "$ISO_PATH"
fi

# Гибридизация для работы на флешках
isohybrid --uefi "$ISO_PATH" || echo "Note: isohybrid finished"

echo "✅ DONE! Created: $ISO_NAME"
echo "--- Final Partition Check ---"
/sbin/fdisk -l "$ISO_PATH"