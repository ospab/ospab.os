/* Linker script for ospabOS kernel - Limine boot protocol */

/* Entry point */
ENTRY(_start)

/* We want to be loaded in the higher half (0xffffffff80000000+) */
KERNEL_PHYS = 0x0000000000200000;  /* Physical load address (2MiB) */
KERNEL_VIRT = 0xffffffff80200000;  /* Virtual address (higher half + 2MiB) */

PHDRS
{
    text    PT_LOAD FLAGS(5);   /* R-X */
    rodata  PT_LOAD FLAGS(4);   /* R-- */
    data    PT_LOAD FLAGS(6);   /* RW- */
}

SECTIONS
{
    . = KERNEL_VIRT;

    /* Text section - executable code */
    .text ALIGN(4K) : AT(ADDR(.text) - KERNEL_VIRT + KERNEL_PHYS)
    {
        *(.text .text.*)
    } :text

    /* Limine requests section - must be scanned by bootloader */
    .limine_requests ALIGN(8) : AT(ADDR(.limine_requests) - KERNEL_VIRT + KERNEL_PHYS)
    {
        KEEP(*(.limine_requests_start))
        KEEP(*(.limine_requests))
        KEEP(*(.limine_requests_end))
    } :text

    /* Read-only data */
    .rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_VIRT + KERNEL_PHYS)
    {
        *(.rodata .rodata.*)
    } :rodata

    /* Data section - initialized read-write data */
    .data ALIGN(4K) : AT(ADDR(.data) - KERNEL_VIRT + KERNEL_PHYS)
    {
        *(.data .data.*)
    } :data

    /* BSS section - uninitialized data */
    .bss ALIGN(4K) : AT(ADDR(.bss) - KERNEL_VIRT + KERNEL_PHYS)
    {
        *(COMMON)
        *(.bss .bss.*)
    } :data

    /* Discard notes and other unnecessary sections */
    /DISCARD/ :
    {
        *(.eh_frame*)
        *(.note .note.*)
        *(.comment)
    }

    /* Provide symbols for kernel use */
    _kernel_start = KERNEL_VIRT;
    _kernel_end = .;
    _kernel_phys_start = KERNEL_PHYS;
}
