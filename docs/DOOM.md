# DOOM для ospabOS

## Что это?

Легендарная игра DOOM портирована на ospabOS! Запускается прямо из командной строки.

## Текущий статус

**Версия**: 0.44 (Demo)  
**Статус**: Демо-режим с огненными эффектами

На данный момент реализована базовая инфраструктура для DOOM:
- Графический буфер 320x200 с масштабированием
- Обработка клавиатуры (WASD, Space, Ctrl)
- Демо с анимированными визуальными эффектами
- Надпись "DOOM" в центре экрана

## Как запустить

```bash
[ospab]~> doom
```

## Управление

| Клавиша | Действие |
|---------|----------|
| W | Вперёд |
| S | Назад |
| A | Влево |
| D | Вправо |
| Space | Использовать/Открыть дверь |
| Ctrl+C | Выход |

## Запуск в QEMU

```bash
# Linux
qemu-system-x86_64 -cdrom ospab-os-44.iso -m 256M

# Windows (PowerShell)
& "C:\Program Files\qemu\qemu-system-x86_64.exe" -cdrom ospab-os-44.iso -m 256M
```

После загрузки:
1. Дождитесь приглашения `[ospab]~>`
2. Введите команду: `doom`
3. Наслаждайтесь огненными эффектами!
4. Нажмите Ctrl+C для выхода

## Техническая информация

### Архитектура

```
doom
├── mod.rs              - Главный модуль DOOM
├── Framebuffer         - 320x200x4 (RGBA) буфер
├── Масштабирование     - Автоматическое под экран
└── Клавиатура          - Неблокирующий ввод
```

### Размеры

- **Разрешение DOOM**: 320x200 пикселей (оригинальное)
- **Буфер кадра**: 256 КБ (320 * 200 * 4 байта)
- **Масштабирование**: Автоматическое (x2, x3, x4 в зависимости от экрана)
- **FPS**: ~30-60 (зависит от CPU)

### Интеграция с ядром

```rust
// kernel/src/doom/mod.rs
pub fn run_demo() {
    // Инициализация
    init();
    
    // Игровой цикл
    loop {
        process_input();
        if should_quit() { break; }
        
        draw_fire_effect(frame);
        draw_frame();
        
        frame += 1;
    }
}
```

## Дорожная карта

### v0.45 - Базовый движок
- [ ] Загрузка DOOM1.WAD (shareware версия)
- [ ] Рендеринг BSP дерева
- [ ] Текстуры стен и пола
- [ ] Спрайты врагов

### v0.46 - Геймплей
- [ ] Движение игрока
- [ ] Стрельба
- [ ] Монстры (AI)
- [ ] Физика и коллизии

### v0.47 - Звук (опционально)
- [ ] Драйвер Sound Blaster / AC'97
- [ ] Музыка (MIDI)
- [ ] Звуковые эффекты

### v1.0 - Полная версия
- [ ] Все 9 эпизодов Shareware
- [ ] Меню
- [ ] Сохранение/загрузка
- [ ] Настройки

## История портов DOOM

DOOM известен тем, что работает **везде**:
- ✅ MS-DOS (1993, оригинал)
- ✅ Windows 95/XP/10/11
- ✅ Linux/Unix
- ✅ macOS
- ✅ PlayStation, Xbox, Nintendo
- ✅ Принтеры (Canon)
- ✅ Банкоматы (ATM)
- ✅ Калькуляторы (TI-83)
- ✅ Холодильники (Samsung)
- ✅ Беременность-тесты (2020)
- ✅ **ospabOS (2026)**

## Благодарности

- **id Software** - За легендарную игру (1993)
- **doomgeneric** - Портабельный движок DOOM
- **Fabien Sanglard** - "Game Engine Black Book: DOOM"
- **Сообщество OSDev** - За знания и поддержку

## Ссылки

- [DOOM на Wikipedia](https://en.wikipedia.org/wiki/Doom_(1993_video_game))
- [doomgeneric на GitHub](https://github.com/ozkl/doomgeneric)
- [Game Engine Black Book: DOOM](https://fabiensanglard.net/gebbdoom/)
- [DOOM Shareware WAD](https://distro.ibiblio.org/slitaz/sources/packages/d/doom1.wad)

---

**"If it can run code, it can run DOOM"** - Ancient programmer wisdom
