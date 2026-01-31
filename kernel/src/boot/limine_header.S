/* Minimal Limine header section placeholder.
   This file places a .limine_header section into the final ELF so Limine
   can detect the kernel. If you have the official Limine header object,
   replace this file with the upstream assembly header. */

.section .limine_header,"a",@progbits
.global limine_header
.align 8
limine_header:
    .quad 0x0
    .quad 0x0
    .quad 0x0
    .quad 0x0

.section .note.GNU-stack,"",%progbits
