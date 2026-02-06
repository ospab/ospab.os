# Исправление клавиатуры и Limine конфигурации

**Дата:** 2 февраля 2026  
**Версии:** ISO #52-60  
**Статус:** ✅ Исправлено

---

## Проблема #1: Клавиатура не работает в QEMU

### Симптомы
- Система грузится
- Framebuffer работает
- Промпт отображается
- Клавиатурный ввод не обрабатывается

### Root Cause
Основной цикл ядра использовал `core::hint::spin_loop()`, который постоянно занимает CPU и **не позволяет процессору обрабатывать прерывания**:

```rust
// НЕПРАВИЛЬНО - блокирует прерывания
loop {
    services::terminal::poll_input();
    // ...
    core::hint::spin_loop(); // ❌ CPU не обрабатывает IRQ
}
```

### Решение
Заменить `spin_loop()` на `x86_64::instructions::hlt()`, который **приостанавливает CPU до следующего прерывания**:

```rust
// ПРАВИЛЬНО - позволяет прерываниям
loop {
    services::terminal::poll_input();
    // ...
    x86_64::instructions::hlt(); // ✅ CPU обрабатывает IRQ
}
```

**Файл:** `kernel/src/main.rs`, строка 354

### Документация
Согласно `production-ready.md`, правильная последовательность инициализации:
1. GDT init
2. IDT init
3. Framebuffer init
4. **Keyboard init (IRQ disabled)**
5. PIT init + enable IRQ0
6. **sti (enable CPU interrupts)**
7. Small delay for timer stabilization
8. **Keyboard enable_hw_irq() - последний шаг**

Эта последовательность **уже была реализована правильно**, но `spin_loop()` блокировал обработку IRQ1.

---

## Проблема #2: Limine не загружает ядро

### Симптомы
```
limine: Loading executable 'boot():/kernel.elf'...
PANIC: limine: Failed to open executable with path 'boot():/kernel.elf'. Is the path correct?
```

### Root Cause Analysis

#### Попытка #1: Неправильный синтаксис конфигурации
```
:ospabOS                    ❌ Entry должен начинаться с /
    KERNEL_PATH=...         ❌ Uppercase + = вместо :
    PROTOCOL=limine         ❌ Uppercase
```

**Ошибка:** `[config file contains no valid entries]`

#### Попытка #2: Неправильный URI
```
/ospabOS                    ✅ Правильно
    protocol: limine        ✅ Правильно
    path: boot:///kernel.elf    ❌ Три слеша
```

**Ошибка:** `Failed to open executable with path 'boot:///kernel.elf'`

#### Попытка #3: Неправильное имя файла
```
/ospabOS                    ✅ Правильно
    protocol: limine        ✅ Правильно
    path: boot():/kernel.elf    ❌ Ядро названо 'kernel', не 'kernel.elf'
```

**Ошибка:** `Failed to open executable with path 'boot():/kernel.elf'`

### Решение
Согласно `kernel/tools/limine/CONFIG.md`:

**Правильный синтаксис:**
```
/Entry Name                 ← Начинается с /
    option_name: value      ← Lowercase, двоеточие + пробел
```

**Правильный URI:**
```
boot():/path/to/file        ← boot() с пустыми скобками = "boot partition"
```

**Правильное имя файла:**
Скрипт `build_with_alloc.sh` копирует:
```bash
cp "$KERNEL_DIR/target/x86_64-ospab/release/ospab-os" iso_root/kernel
```

**Итоговая конфигурация:**
```
timeout: 5
default_entry: 1

/ospabOS
    protocol: limine
    path: boot():/kernel
```

**Файл:** `kernel/iso_root/limine.conf`

---

## Проблема #3: UEFI boot не работал

### Решение
Добавлен UEFI support в `build_with_alloc.sh`:

1. **ESP FAT image:**
```bash
dd if=/dev/zero of="iso_root/$ESP_IMG" bs=1M count=20
mkfs.vfat -n ESP "iso_root/$ESP_IMG"
```

2. **Копирование BOOTX64.EFI:**
```bash
mcopy -i "iso_root/$ESP_IMG" "$LIMINE_BIN_DIR/BOOTX64.EFI" ::/$UEFI_DIR/BOOTX64.EFI
```

3. **El Torito boot catalog:**
```bash
xorriso -as mkisofs \
    -b limine-bios-cd.bin \
    -c boot.cat \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    -eltorito-alt-boot \
    -e "$ESP_IMG" -no-emul-boot \
    iso_root -o "$ISOS_DIR/$ISO_NAME"
```

---

## Проблема #4: Ошибки компиляции build script

### Решение
Добавлен `cd "$KERNEL_DIR"` перед `cargo build`:

```bash
echo "Building ospabOS kernel..."
cd "$KERNEL_DIR"  # ← Добавлено
cargo +nightly build --release -Z build-std=core,alloc --target x86_64-ospab.json
```

**Файл:** `kernel/build_with_alloc.sh`, строка 57

---

## Итоговые изменения

### Файлы изменены:
1. **kernel/src/main.rs** (строка 354)
   - `core::hint::spin_loop()` → `x86_64::instructions::hlt()`

2. **kernel/iso_root/limine.conf**
   ```
   timeout: 5
   default_entry: 1
   
   /ospabOS
       protocol: limine
       path: boot():/kernel
   ```

3. **kernel/build_with_alloc.sh**
   - Добавлен `cd "$KERNEL_DIR"` перед cargo build
   - Добавлена поддержка UEFI (ESP FAT image)
   - El Torito boot catalog с `-c boot.cat`
   - BOOTIA32.EFI сделан опциональным

### ISO версии:
- **#52-55**: Попытки с неправильной конфигурацией Limine
- **#56-59**: Исправление синтаксиса и URI
- **#60**: ✅ Рабочая версия (BIOS+UEFI hybrid, клавиатура работает)

---

## Тестирование

### QEMU (BIOS):
```powershell
& "d:\Toolz\qemu\qemu-system-x86_64.exe" -cdrom kernel/isos/ospab-os-60.iso -m 256M
```

**Результат:** ✅ Загружается, клавиатура работает

### Реальное железо (CDROM):
**Ожидается:** Должно грузиться с CD/DVD без ошибок `code 0009`

---

## Lessons Learned

1. **RTFM!** - Всегда проверяй документацию (`CONFIG.md`) перед угадыванием синтаксиса
2. **hlt() vs spin_loop()** - `spin_loop()` для spinlocks, `hlt()` для ожидания прерываний
3. **URI schemes** - `boot()` ≠ `boot:///` в Limine
4. **Имена файлов** - Проверяй что копируется в ISO и что указано в конфиге
5. **Working directory** - Cargo должен запускаться из директории с `Cargo.toml`

---

## References
- `kernel/tools/limine/CONFIG.md` - Limine configuration syntax
- `production-ready.md` - Keyboard initialization sequence
- `kernel/src/main.rs` - Main event loop
- `kernel/build_with_alloc.sh` - ISO build script
