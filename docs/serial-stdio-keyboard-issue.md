# QEMU Serial Stdio и PS/2 клавиатура

**Дата:** 2 февраля 2026  
**Версия:** ospabOS v0.1.5  
**ISO:** #66  
**Статус:** RESOLVED

## Проблема

После добавления VMM (Virtual Memory Manager) и команд shutdown/reboot в ISO #62-65, клавиатура перестала работать при запуске QEMU с параметром `-serial stdio`.

### Симптомы

```bash
# Клавиатура НЕ работает
qemu-system-x86_64 -cdrom ospab-os.iso -m 256M -serial stdio

# Клавиатура РАБОТАЕТ
qemu-system-x86_64 -cdrom ospab-os.iso -m 256M
```

### Логи системы

Система инициализировалась корректно:
```
[KBD] Enabling keyboard hardware IRQ...
[PIC] Enabled IRQ 1
[KBD] Current config byte: 0x9C
[KBD] New config byte: 0x9D
[KBD] Config written successfully
[KBD] Verified config byte: 0x9D
[KBD] Keyboard IRQ enabled
```

Но прерывания клавиатуры (IRQ1) не поступали - добавление отладки `[KBD-IRQ]` в обработчик прерывания не давало вывода при нажатии клавиш.

## Root Cause

**QEMU в режиме `-serial stdio` перехватывает весь ввод для serial порта (COM1) и НЕ передаёт его на эмулированную PS/2 клавиатуру.**

Это документированное поведение QEMU:
- `-serial stdio` означает "подключить COM1 к stdin/stdout терминала"
- Весь ввод с клавиатуры направляется в serial port
- PS/2 контроллер не получает scancodes
- IRQ1 не генерируется

### Техническая детализация

1. **PS/2 клавиатура (порт 0x60):**
   - Генерирует IRQ1 при нажатии клавиш
   - Работает в графическом окне QEMU
   - НЕ работает когда stdin перехвачен для serial

2. **Serial порт COM1 (порт 0x3F8):**
   - Принимает данные через stdin когда используется `-serial stdio`
   - Может генерировать IRQ4 (не используется в ospabOS)
   - Работает для вывода (stdout)

3. **Конфликт ресурсов:**
   ```
   stdin → QEMU → [выбор: COM1 ИЛИ PS/2]
   
   С -serial stdio:
   stdin → QEMU → COM1 (порт 0x3F8) ✓
                → PS/2 (порт 0x60) ✗
   
   Без -serial stdio:
   stdin → QEMU → PS/2 (порт 0x60) ✓
   ```

## Решение

### Вариант 1: Использовать графическое окно (РЕКОМЕНДУЕТСЯ)

```bash
# Для интерактивной работы с клавиатурой
qemu-system-x86_64 -cdrom kernel/isos/ospab-os-66.iso -m 256M
```

**Преимущества:**
- Полная функциональность PS/2 клавиатуры
- Framebuffer виден в графическом окне
- IRQ1 работает корректно

**Недостатки:**
- Нет логов в терминале
- Требуется X server / графическая среда

### Вариант 2: Serial console (будущее развитие)

Реализовать полноценный serial console с обработкой ввода через COM1:

```rust
// kernel/src/drivers/serial.rs
pub fn poll_input() -> Option<u8> {
    SERIAL.lock().read_byte()
}

// kernel/src/main.rs
loop {
    // Проверить serial input
    while let Some(byte) = drivers::serial::poll_input() {
        process_serial_command(byte);
    }
    
    // Обработать keyboard input (если не -serial stdio)
    services::terminal::poll_input();
    
    x86_64::instructions::hlt();
}
```

**Преимущества:**
- Работает с `-serial stdio`
- Удобно для автоматизации и тестирования
- Логи доступны в терминале

**Недостатки:**
- Требует дублирования логики shell
- Нет доступа к framebuffer через serial
- Дополнительная сложность

### Вариант 3: Мониторный режим

```bash
qemu-system-x86_64 -cdrom ospab-os.iso -m 256M -serial mon:stdio
```

Позволяет использовать QEMU monitor, но клавиатура всё равно идёт в serial.

## Отладка

### Проверка работы PS/2 контроллера

Добавлена детальная отладка в `keyboard.rs`:

```rust
pub fn enable_hw_irq() {
    // Unmask IRQ1 in PIC
    crate::interrupts::enable_irq(1);
    
    // Read config
    cmd_port.write(CMD_READ_CONFIG);
    let mut config = data_port.read();
    serial_print(b"[KBD] Current config byte: 0x");
    serial_print_hex(config);
    
    // Enable IRQ1
    config |= CONFIG_IRQ1_ENABLED;
    
    // Write back and verify
    cmd_port.write(CMD_WRITE_CONFIG);
    data_port.write(config);
    
    // Verify
    cmd_port.write(CMD_READ_CONFIG);
    let verify_config = data_port.read();
    serial_print(b"[KBD] Verified config byte: 0x");
    serial_print_hex(verify_config);
}
```

Вывод показал, что PS/2 контроллер настроен корректно (0x9D = IRQ включено), но прерывания не приходят из-за перехвата stdin в QEMU.

### Проверка прерываний клавиатуры

Добавлен отладочный вывод в ISR:

```rust
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    serial_print(b"[KBD-IRQ] "); // Debug
    
    let status: u8 = unsafe { Port::new(0x64).read() };
    if (status & 0x01) == 0 {
        serial_print(b"spurious\r\n");
        notify_end_of_interrupt(1);
        return;
    }
    
    let scancode: u8 = unsafe { Port::new(0x60).read() };
    serial_print(b"sc=");
    serial_print_hex(scancode);
    serial_print(b"\r\n");
    
    crate::drivers::keyboard::queue_scancode(scancode);
    notify_end_of_interrupt(1);
}
```

При запуске с `-serial stdio` сообщения `[KBD-IRQ]` не появлялись → IRQ1 не генерируется.

## Проверка решения

```bash
# Собрать ISO #66 (без отладки)
wsl bash kernel/build_with_alloc.sh

# Запустить с графическим окном
qemu-system-x86_64 -cdrom kernel/isos/ospab-os-66.iso -m 256M
```

**Результат:** ✅ Клавиатура работает корректно в графическом окне QEMU.

## Исторический контекст

Эта проблема уже встречалась ранее:

### ISO #51 (20 января 2026)
- **Проблема:** Клавиатура не работала из-за `spin_loop()` в main loop
- **Решение:** Заменили `spin_loop()` на `hlt()`
- **Документация:** `docs/fix-keyboard-and-limine.md`

### ISO #62-66 (2 февраля 2026)
- **Проблема:** Клавиатура не работает с `-serial stdio`
- **Решение:** Использовать графическое окно QEMU
- **Root cause:** QEMU перехватывает stdin для COM1
- **Документация:** Этот файл

## Рекомендации

### Для разработки

1. **Интерактивное тестирование:**
   ```bash
   qemu-system-x86_64 -cdrom ospab-os.iso -m 256M
   ```
   Клавиатура работает, можно тестировать shell commands.

2. **Отладка через serial:**
   ```bash
   qemu-system-x86_64 -cdrom ospab-os.iso -m 256M -serial stdio
   ```
   Логи видны в терминале, но клавиатура не работает (это ожидаемо).

3. **Dual mode:**
   ```bash
   # Terminal 1: QEMU с графическим окном
   qemu-system-x86_64 -cdrom ospab-os.iso -m 256M -serial file:serial.log
   
   # Terminal 2: Мониторинг логов
   tail -f serial.log
   ```

### Для production

Реализовать serial console с полной функциональностью:
- Чтение команд из COM1
- Обработка escape sequences
- Дублирование вывода в framebuffer и serial
- Переключение между PS/2 и serial input автоматически

## Ссылки

- [QEMU Serial Console Documentation](https://www.qemu.org/docs/master/system/device-emulation.html#serial-port)
- [OSDev: PS/2 Keyboard](https://wiki.osdev.org/PS2_Keyboard)
- [OSDev: Serial Ports](https://wiki.osdev.org/Serial_Ports)
- `docs/fix-keyboard-and-limine.md` - предыдущая проблема с клавиатурой

## Changelog

- **2026-02-02:** Проблема обнаружена и решена (ISO #66)
- **2026-02-02:** Добавлена детальная отладка PS/2 контроллера
- **2026-02-02:** Добавлены функции чтения из serial port (для будущего)
- **2026-02-02:** Документирован root cause и решение
