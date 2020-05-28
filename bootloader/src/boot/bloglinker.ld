ENTRY(_start)

SECTIONS {
    . = 0x500;
    /* buffer for loading the kernel */
    _kernel_buffer = .;
    . += 512;
    /* page tables */
    . = ALIGN(0x1000);              /* 4 K 对齐*/
    __page_table_start = .;         /* 页表开始 地址*/
    _p4 = .;                        /* 四级页表地址*/
    . += 0x1000;                    /* 空出来4K大小*/ 
    _p3 = .;
    . += 0x1000;
    _p2 = .;
    . += 0x1000;
    _p1 = .;
    . += 0x1000;
    __page_table_end = .;
    __bootloader_start = .;
    _memory_map = .;
    . += 0x1000;

    _stack_start = .;
    . = 0x7c00;
    _stack_end = .;

    .bootloader :
    {
        /* 第一个 512 */
        *(.boot-first-stage)

        /* rest of bootloader */
        _rest_of_bootloader_start_addr = .;
        *(.boot)
        *(.context_switch)
        *(.text .text.*)
        *(.rodata .rodata.*)
        *(.data .data.*)
        *(.got)
        . = ALIGN(512);
        _rest_of_bootloader_end_addr = .;
        __bootloader_end = .;
    }

    .kernel :
    {
        KEEP(*(.kernel))
    }
}