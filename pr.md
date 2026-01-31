SYSTEM INSTRUCTION (IDENTITY v.5.0): > Ты — Senior Systems Programmer. Прекрати выдавать код по частям. Если ты начинаешь обновление, ты обязан выдать ПОЛНУЮ реализацию всех затронутых файлов в одном ответе. No placeholders, no TODOs.

CONTEXT: > 1. Bootloader: Мы используем Limine 10.x. Ошибка: [config file not found]. 2. Kernel Panic: Ошибка PageAlreadyMapped на адресе 0x0 в page_table.rs. 3. Goal: Получить работающий Shell и корректную загрузку через ISO.

TASK 1: ISO Structure & Build Script Напиши Bash-скрипт (или инструкции для Cargo), который:

    Создает структуру папок для ISO: iso_root/boot/, iso_root/EFI/BOOT/.

    Копирует скомпилированный бинарник ядра в iso_root/boot/kernel.elf.

    Копирует файл limine.conf (именно .conf!) в корень iso_root/.

    Использует xorriso для сборки финального .iso образа, который будет работать и в BIOS, и в UEFI.

TASK 2: Limine Configuration Напиши идеальный limine.conf. Учти, что путь к ядру должен быть boot:///boot/kernel.elf. Установи таймаут 0 или 3 секунды.

TASK 3: Full Code Implementation (DO NOT STOP) Выдай полный, готовый к работе код для следующих файлов:

    src/page_table.rs: Реализуй проверку: если страница уже смаппирована (особенно 0x0), и целевой физический адрес совпадает с текущим — возвращай Ok(()) без паники. Это критически важно для совместимости с Limine Memory Map.

    src/interrupts.rs: Полная IDT. Обработчик клавиатуры (IRQ 1) должен складывать символы в защищенный мьютексом буфер.

    src/shell.rs: Реализуй логику терминала. Команды: help, clear, ping. Добавь поддержку Backspace и прокрутку текста.

    src/main.rs: Инициализация GDT, IDT, включение прерываний и запуск бесконечного цикла Shell.

CRITICAL RULE: Если код не влезает в лимит сообщения, остановись на границе файла, и когда я скажу "Continue", продолжи СЛЕДУЮЩИЙ файл. Не сокращай логику!