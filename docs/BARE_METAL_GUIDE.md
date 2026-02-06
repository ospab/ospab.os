# ospabOS - Bare Metal Installation Guide

## English Version

### System Requirements

**Minimum Hardware:**
- x86_64 CPU (Intel/AMD 64-bit)
- 128 MB RAM (recommended 256 MB+)
- VGA-compatible graphics (framebuffer support)
- PS/2 or USB keyboard
- UEFI or Legacy BIOS firmware

**Tested Hardware:**
- QEMU/KVM virtual machines
- Intel/AMD processors with SSE2 support
- UEFI firmware version 2.0+

### Pre-Installation Checklist

1. ✅ **UEFI Boot Support**: ospabOS uses Limine bootloader with UEFI support
2. ✅ **Serial Port**: COM1 (optional, for debugging)
3. ✅ **Framebuffer**: VGA or UEFI GOP (Graphics Output Protocol)
4. ✅ **Keyboard**: PS/2 or USB HID

### Installation Methods

#### Method 1: USB Drive (Recommended for Real Hardware)

**Requirements:**
- USB drive (512 MB minimum)
- Linux machine with `dd` command
- `ospab-os-XX.iso` file from `kernel/isos/`

**Steps:**

```bash
# 1. Find USB drive device
lsblk

# 2. Write ISO to USB (CAUTION: This will erase all data!)
sudo dd if=ospab-os-43.iso of=/dev/sdX bs=4M status=progress
sudo sync

# 3. Safely eject
sudo eject /dev/sdX
```

**Windows Steps:**

1. Download Rufus: https://rufus.ie/
2. Select ospab-os-XX.iso
3. Select USB drive
4. Choose "DD Image" mode
5. Click Start

#### Method 2: CD/DVD

**Requirements:**
- Blank CD-R or DVD-R
- CD/DVD burning software

**Steps:**

1. **Linux:** `wodim -v dev=/dev/sr0 ospab-os-43.iso`
2. **Windows:** Use ImgBurn or similar
3. **macOS:** Use Disk Utility → Burn

#### Method 3: GRUB Chainload (Advanced)

Add to existing GRUB config (`/boot/grub/grub.cfg`):

```
menuentry "ospabOS" {
    set root=(hd0,1)
    linux /boot/ospab/kernel.elf
    boot
}
```

### Booting ospabOS

#### UEFI Boot Process

1. **Power On** → UEFI firmware initializes
2. **Limine Bootloader** loads from ESP (EFI System Partition)
3. **Kernel Detection** → Limine finds `/kernel.elf`
4. **Memory Map** → UEFI provides memory regions
5. **Framebuffer Setup** → GOP mode initialization
6. **Kernel Entry** → ospabOS kernel starts

#### Boot Menu Options

Limine presents a boot menu with 5-second timeout:
- **default**: Boot ospabOS normally
- Serial console enabled at 115200 baud

### First Boot

**Expected Behavior:**

1. **Limine Banner**: "Limine 10.6.3 (x86-64, BIOS/UEFI)"
2. **Boot Selection**: "ospabOS v0.38 - Unix-like VFS + Grape"
3. **Kernel Messages**: Serial output shows initialization
4. **Framebuffer Welcome**: White text on black background
5. **Shell Prompt**: `[ospab]~> `

**Serial Console Output:**

Connect serial cable to COM1 (115200 8N1):
```
         ospabOS Kernel v0.38.0        
========================================
[1/7] Checking Limine protocol... OK
[2/7] Getting HHDM offset... OK
[3/7] Initializing GDT... OK
[4/7] Initializing IDT and PICs... OK
[5/7] Initializing framebuffer... OK
[6/8] Initializing serial port... OK
[7/8] Initializing keyboard driver... OK
[8/8] All components initialized
```

### Verifying Installation

Run these commands to test functionality:

```bash
# Check version
version

# Test filesystem
ls
ls /bin
ls /etc
cat /etc/hostname
cat /etc/os-release

# Test navigation
cd /etc
pwd
cd ..
pwd

# Check system uptime
uptime

# Test text editor
grape test.txt
# Use: Ctrl+X to save, Ctrl+C to exit

# View command history
history
```

### Troubleshooting

#### Problem: Black Screen After Boot

**Cause**: Framebuffer initialization failed

**Solution:**
- Try different display mode in BIOS/UEFI
- Enable CSM (Compatibility Support Module) if available
- Check serial console for error messages

#### Problem: Keyboard Not Working

**Cause**: USB keyboard not recognized

**Solution:**
- Use PS/2 keyboard
- Enable "USB Legacy Support" in BIOS
- Try different USB port (preferably USB 2.0)

#### Problem: System Hangs at "Enabling CPU interrupts"

**Cause**: Timer or PIC misconfiguration

**Solution:**
- Disable APIC in BIOS
- Enable "Legacy IRQ" support
- Check serial output for specific error

#### Problem: "No bootable device" Error

**Cause**: UEFI can't find bootloader

**Solution:**
- Verify USB is bootable (use Rufus in DD mode)
- Enable UEFI boot in BIOS (disable Secure Boot)
- Try Legacy/CSM boot mode

### Serial Console Access

**Hardware Setup:**
- Null-modem cable between COM1 and another PC
- USB-to-Serial adapter

**Software:**
- **Linux:** `screen /dev/ttyUSB0 115200`
- **Windows:** PuTTY (115200 8N1)
- **macOS:** `screen /dev/cu.usbserial 115200`

### Performance Tips

1. **RAM**: More RAM = better performance (no disk swapping yet)
2. **CPU**: SSE2 required, SSE3+ recommended
3. **Graphics**: Native resolution recommended
4. **Storage**: OS runs entirely in RAM (no disk writes yet)

### Known Limitations

- **Read-Only Filesystem**: Cannot write files to disk (only in-memory)
- **No Networking**: Network stack not implemented
- **No USB Storage**: Only keyboard input supported
- **No Sound**: Audio drivers not implemented
- **Single-Core**: No SMP support yet

### Hardware Compatibility List

**Confirmed Working:**
- Intel Core i3/i5/i7 (2nd gen+)
- AMD Ryzen series
- VirtualBox 6.0+
- VMware Workstation 15+
- QEMU/KVM 4.0+

**Known Issues:**
- Some Atom processors (SSE2 check fails)
- Very old UEFI implementations (pre-2012)

---

## Русская Версия

### Системные Требования

**Минимальное оборудование:**
- Процессор x86_64 (Intel/AMD 64-bit)
- 128 МБ RAM (рекомендуется 256 МБ+)
- VGA-совместимая графика (поддержка framebuffer)
- PS/2 или USB клавиатура
- Прошивка UEFI или Legacy BIOS

**Протестировано на:**
- Виртуальные машины QEMU/KVM
- Процессоры Intel/AMD с поддержкой SSE2
- UEFI прошивка версии 2.0+

### Предварительная Проверка

1. ✅ **Поддержка UEFI**: ospabOS использует загрузчик Limine с UEFI
2. ✅ **Последовательный порт**: COM1 (опционально, для отладки)
3. ✅ **Framebuffer**: VGA или UEFI GOP
4. ✅ **Клавиатура**: PS/2 или USB HID

### Методы Установки

#### Метод 1: USB-накопитель (Рекомендуется)

**Требования:**
- USB-флешка (минимум 512 МБ)
- Linux с командой `dd`
- Файл `ospab-os-XX.iso` из `kernel/isos/`

**Шаги:**

```bash
# 1. Найти USB устройство
lsblk

# 2. Записать ISO на USB (ВНИМАНИЕ: Удалит все данные!)
sudo dd if=ospab-os-43.iso of=/dev/sdX bs=4M status=progress
sudo sync

# 3. Безопасное извлечение
sudo eject /dev/sdX
```

**Для Windows:**

1. Скачать Rufus: https://rufus.ie/
2. Выбрать ospab-os-XX.iso
3. Выбрать USB-накопитель
4. Режим "DD образ"
5. Нажать Start

### Загрузка ospabOS

#### Процесс UEFI загрузки

1. **Включение** → Инициализация UEFI
2. **Загрузчик Limine** с ESP раздела
3. **Обнаружение ядра** → `/kernel.elf`
4. **Карта памяти** → UEFI предоставляет регионы
5. **Framebuffer** → Инициализация GOP
6. **Запуск ядра** → Старт ospabOS

### Первая Загрузка

**Ожидаемое поведение:**

1. **Баннер Limine**: "Limine 10.6.3"
2. **Меню загрузки**: "ospabOS v0.38"
3. **Сообщения ядра**: Вывод в serial
4. **Приветствие**: Белый текст на черном фоне
5. **Приглашение shell**: `[ospab]~> `

### Проверка Установки

Команды для тестирования:

```bash
# Версия системы
version

# Тест файловой системы
ls
ls /bin
cat /etc/hostname

# Навигация
cd /etc
pwd

# Время работы
uptime

# Текстовый редактор
grape test.txt
# Ctrl+X сохранить, Ctrl+C выход
```

### Устранение Проблем

#### Черный Экран

**Причина**: Ошибка framebuffer

**Решение:**
- Другой режим дисплея в BIOS
- Включить CSM
- Проверить serial консоль

#### Клавиатура Не Работает

**Причина**: USB не распознается

**Решение:**
- Использовать PS/2 клавиатуру
- "USB Legacy Support" в BIOS
- Другой USB порт (USB 2.0)

### Ограничения

- **Файловая система только для чтения**
- **Нет сети**
- **Нет USB хранилища**
- **Нет звука**
- **Один процессор**

### Совместимость

**Работает:**
- Intel Core i3/i5/i7
- AMD Ryzen
- VirtualBox 6.0+
- VMware 15+
- QEMU/KVM 4.0+

---

## Next Steps

After successful boot, explore:

1. **Shell Commands**: `help`, `ls`, `cat`, `cd`, `pwd`, `uptime`
2. **File Editor**: `grape <filename>` - nano-like editor
3. **File System**: Unix-like hierarchy (`/bin`, `/etc`, `/home`, `/dev`)
4. **Development**: See `ARCHITECTURE.md` for kernel structure

## Support

- GitHub: [ospab-projects/ospab.os]
- Documentation: See `README.md` and `ARCHITECTURE.md`
- Serial Debug: COM1 115200 8N1
