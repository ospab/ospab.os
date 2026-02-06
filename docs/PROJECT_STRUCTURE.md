# 📁 Структура проекта ospabOS

Обновлено: 2 февраля 2026  
Версия: 0.44 "DOOM Edition"

## 📚 Документация

### Основная документация

| Файл | Описание | Язык |
|------|----------|------|
| [README.md](README.md) | Главная документация проекта | English |
| [README_RU.md](README_RU.md) | Главная документация проекта | Русский |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Техническая архитектура | English |
| [CHANGELOG.md](CHANGELOG.md) | История версий и roadmap | English |

### Специальная документация

| Файл | Описание | Язык |
|------|----------|------|
| [DOOM.md](DOOM.md) | DOOM порт - руководство | Русский |
| [DOOM_EN.md](DOOM_EN.md) | DOOM port - guide | English |
| [BARE_METAL_GUIDE.md](BARE_METAL_GUIDE.md) | Установка на железо | EN + RU |
| [TESTING.md](TESTING.md) | Инструкции по тестированию | Русский |

### Скрипты установки

| Файл | Описание | Платформа |
|------|----------|-----------|
| [install_usb.sh](install_usb.sh) | Создание загрузочного USB | Linux/macOS |
| [install_usb.ps1](install_usb.ps1) | Создание загрузочного USB | Windows |

## 🗂️ Структура исходников

```
ospab.os/
├── 📚 Документация (корень)
│   ├── README.md                  Главная (EN)
│   ├── README_RU.md               Главная (RU)
│   ├── ARCHITECTURE.md            Архитектура
│   ├── CHANGELOG.md               История версий
│   ├── DOOM.md                    DOOM (RU)
│   ├── DOOM_EN.md                 DOOM (EN)
│   ├── BARE_METAL_GUIDE.md        Установка (EN/RU)
│   └── TESTING.md                 Тестирование
│
├── 🔧 Установка
│   ├── install_usb.sh             USB скрипт (Linux)
│   └── install_usb.ps1            USB скрипт (Windows)
│
├── 💿 Образы (корень)
│   ├── ospab-os.iso               Последний ISO (symlink)
│   ├── iso_root/                  Корень ISO (сборка)
│   └── iso_extract/               Распакованный ISO
│
├── 🛠️ Конфигурация
│   ├── .cargo/                    Cargo config
│   ├── x86_64-os.json             Target spec (старый)
│   └── x86_64-ospab_os.json       Target spec
│
├── 📝 Логи и заметки
│   ├── review.md                  Обзор разработки
│   ├── pr.md                      Pull request
│   ├── fix.md                     Исправления
│   ├── popa.md                    Заметки
│   ├── production-ready.md        Production notes
│   ├── serial-26.log              Серийный лог #26
│   ├── serial2.log                Серийный лог #2
│   ├── qemu_out.txt               QEMU stdout
│   └── qemu_err.txt               QEMU stderr
│
├── 📦 kernel/ (ОСНОВНАЯ ДИРЕКТОРИЯ)
│   ├── src/                       Исходники ядра
│   │   ├── main.rs                Entry point
│   │   ├── lib.rs                 Module declarations
│   │   ├── arch/                  Архитектура x86_64
│   │   │   └── x86_64/
│   │   │       ├── gdt.rs         Global Descriptor Table
│   │   │       └── idt.rs         Interrupt Descriptor Table
│   │   ├── drivers/               Драйверы
│   │   │   ├── framebuffer.rs     VGA/UEFI дисплей
│   │   │   ├── keyboard.rs        PS/2 клавиатура
│   │   │   ├── timer.rs           PIT таймер
│   │   │   ├── serial.rs          COM1 serial
│   │   │   └── font_data.rs       8x8 font
│   │   ├── ipc/                   IPC шина сообщений
│   │   │   ├── bus.rs             Message bus
│   │   │   └── message.rs         Message types
│   │   ├── services/              Системные сервисы
│   │   │   ├── vfs.rs             Virtual filesystem
│   │   │   └── terminal.rs        Terminal service
│   │   ├── doom/                  DOOM модуль ⭐
│   │   │   └── mod.rs             DOOM реализация
│   │   ├── grape/                 Текстовый редактор
│   │   │   └── mod.rs             Grape editor
│   │   ├── shell/                 Командная оболочка
│   │   │   └── mod.rs             Shell interpreter
│   │   ├── interrupts.rs          Interrupt handlers
│   │   ├── boot.rs                Limine protocol
│   │   ├── allocator.rs           Heap allocator
│   │   ├── mem.rs                 Memory management
│   │   ├── task.rs                Task management
│   │   ├── process.rs             Process management
│   │   ├── gdt.rs                 GDT setup
│   │   ├── sync.rs                Synchronization
│   │   └── common.rs              Common utilities
│   │
│   ├── isos/                      Собранные ISO образы
│   │   ├── ospab-os-44.iso        v0.44 (DOOM Edition) ⭐
│   │   ├── ospab-os-43.iso        v0.43
│   │   ├── ospab-os-42.iso        v0.42
│   │   └── ...                    Старые версии
│   │
│   ├── iso_root/                  Временная директория ISO
│   │   ├── kernel.elf             Скомпилированное ядро
│   │   ├── limine-bios.sys        Limine BIOS
│   │   ├── limine-bios-cd.bin     Limine CD boot
│   │   ├── limine-uefi-cd.bin     Limine UEFI CD
│   │   ├── limine.cfg             Конфиг загрузчика
│   │   └── EFI/                   UEFI boot
│   │       └── BOOT/
│   │           ├── BOOTX64.EFI    UEFI загрузчик
│   │           └── limine.cfg     UEFI конфиг
│   │
│   ├── initrd/                    Initial RAM disk
│   │   ├── bin/                   Системные команды
│   │   │   ├── ls                 List directory
│   │   │   ├── cat                Display file
│   │   │   └── grape              Text editor
│   │   ├── etc/                   Конфигурация
│   │   │   ├── hostname           System name
│   │   │   └── os-release         Version info
│   │   └── home/
│   │       └── user/              User home
│   │
│   ├── Cargo.toml                 Rust project config
│   ├── Cargo.lock                 Dependency lock
│   ├── build_with_alloc.sh        Скрипт сборки (Linux) ⭐
│   ├── x86_64-ospab.json          Custom target spec
│   └── limine.cfg                 Limine config
│
└── 🍅 tomato-pm/                  Package manager (future)
    └── ...
```

## 🔑 Ключевые файлы

### Для сборки

1. **kernel/build_with_alloc.sh** - Главный скрипт сборки
   - Компилирует Rust → kernel.elf
   - Создаёт ISO с Limine
   - Выводит ospab-os-XX.iso

2. **kernel/Cargo.toml** - Конфигурация Rust
   - Dependencies: pc-keyboard, spin, linked_list_allocator
   - Profile: release с оптимизациями

3. **kernel/src/main.rs** - Entry point
   - Инициализация GDT, IDT
   - Запуск драйверов
   - Main loop

### Для DOOM

1. **kernel/src/doom/mod.rs** - DOOM реализация
   - Framebuffer 320x200
   - Keyboard input
   - Fire effect demo
   - Game loop

2. **kernel/src/shell/mod.rs** - Shell с командой `doom`
   - Запуск DOOM из shell
   - Обработка команд

3. **kernel/src/drivers/framebuffer.rs** - Graphics API
   - `set_pixel()` - Рисование пикселей
   - `get_info()` - Информация о буфере

4. **kernel/src/drivers/keyboard.rs** - Input API
   - `try_read_key()` - Неблокирующий ввод

## 📦 Собранные образы

Все ISO образы находятся в `kernel/isos/`:

| ISO | Версия | Размер | Дата | Описание |
|-----|--------|--------|------|----------|
| ospab-os-44.iso | v0.44 | 1.4 MB | 02.02.2026 | **DOOM Edition** ⭐ |
| ospab-os-43.iso | v0.43 | 1.4 MB | 01.02.2026 | Grape fixes |
| ospab-os-42.iso | v0.42 | 1.4 MB | 31.01.2026 | Cursor fix |
| ospab-os-41.iso | v0.41 | 1.4 MB | 30.01.2026 | Arrow keys |

Последняя стабильная версия: **v0.44**

## 🚀 Как собрать

### Быстрая сборка

```bash
cd kernel
wsl bash build_with_alloc.sh
```

Результат: `kernel/isos/ospab-os-44.iso`

### Запуск

```bash
cd kernel/isos
qemu-system-x86_64 -cdrom ospab-os-44.iso -m 256M -serial stdio
```

### Запуск DOOM

```bash
[ospab]~> doom
# Наслаждайтесь огненными эффектами!
# Ctrl+C для выхода
```

## 📊 Статистика проекта

### Размеры

- **Kernel ELF**: ~800 KB
- **ISO образ**: 1.4 MB (696 sectors)
- **Исходники**: ~50 файлов, ~15000 строк Rust
- **Документация**: 8 файлов, ~3500 строк Markdown

### Модули

- **Драйверы**: 5 (framebuffer, keyboard, timer, serial, font)
- **Сервисы**: 2 (VFS, terminal)
- **IPC**: 2 модуля (bus, message)
- **Приложения**: 3 (shell, grape, doom)
- **Утилиты**: ~15 функций

### Функции

- ✅ Загрузка через UEFI/BIOS
- ✅ Framebuffer console
- ✅ PS/2 клавиатура
- ✅ VFS с Unix-иерархией
- ✅ Текстовый редактор
- ✅ Командная оболочка
- ✅ **DOOM demo** ⭐
- ❌ Persistent storage (v0.48)
- ❌ Networking (v0.49)
- ❌ Sound (v0.47)
- ❌ Multitasking (v1.0)

## 🎯 Следующие шаги

### Для разработчиков

1. Прочитайте [ARCHITECTURE.md](ARCHITECTURE.md)
2. Изучите [kernel/src/doom/mod.rs](kernel/src/doom/mod.rs)
3. Посмотрите [CHANGELOG.md](CHANGELOG.md) для roadmap
4. Соберите проект и протестируйте

### Для пользователей

1. Скачайте `ospab-os-44.iso` из `kernel/isos/`
2. Запустите в QEMU или записывайте на USB
3. Следуйте [TESTING.md](TESTING.md)
4. Наслаждайтесь DOOM! 🎮

## 📧 Контакты

- **Проект**: ospab-projects/ospab.os
- **Версия**: 0.44 "DOOM Edition"
- **Лицензия**: Educational (free to use)
- **Статус**: Active development

---

**"If it can run code, it can run DOOM"** 🔥

Последнее обновление: 2 февраля 2026
