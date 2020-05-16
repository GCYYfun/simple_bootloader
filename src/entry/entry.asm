.code32
.section .text
.global _kstart
_kstart:
    mov $bootstacktop,%esp
    call kmain

.section .bss.stack
.p2align 12
bootstack:
    .space 4096 * 4

.global bootstacktop
bootstacktop: