OUTPUT_FORMAT("elf64-x86-64", "elf64-x86-64", "elf64-x86-64")
OUTPUT_ARCH(i386:x86-64)
ENTRY(_start)

SECTIONS
{
    . = 0x7c00;

    .text : {
        *(.text .text.*)
    }
    .rodata : {
       .rodata .rodata.*
	}
}