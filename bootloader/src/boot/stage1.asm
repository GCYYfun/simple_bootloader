.section .boot-first-stage,"awx"
.global _start
.intel_syntax noprefix
.code16

# 这个阶段初始化堆栈，启用A20行，从磁盘上加载其余的引导程序，并跳转到 stage_2。

_start:
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    cld

    mov sp , 0x7c00

    lea si , boot_start_str
    call real_mode_println

enable_a20:
    in al, 0x92
    test al, 2
    jnz enable_a20_after
    or al, 2
    and al, 0xFE
    out 0x92, al
enable_a20_after:

# 执行 保护模式 的操作 
enter_protected_mode:
    # 关中断
    cli
    push ds
    push es

    # GDTR 设置 gdt
    lgdt [gdt32info]

    # CR0 标志位 pe置位
    mov eax, cr0
    or al, 1 
    mov cr0, eax

    jmp protected_mode     

protected_mode:
    # 设置段寄存器
    mov bx, 0x10
    mov ds, bx # set data segment
    mov es, bx # set extra segment

    and al, 0xfe    # clear protected mode bit
    mov cr0, eax

unreal_mode:
    pop es # get back old extra segment
    pop ds # get back old data segment
    sti

    # 返回到真实模式，但内部数据段寄存器仍然加载有GDT段 -> 我们可以访问整个4GiB内存

    mov bx, 0x0f01         # 有一个笑脸 表示重新回到实模式
    mov eax, 0xb8f00       # note 32 bit offset
    mov word ptr ds:[eax], bx

# 当您要使用的操作系统或应用程序需要访问硬盘时，它会使用BIOS服务来执行此操作。BIOS的主要接口是称为Int13h的软件中断。
check_int13h_extensions:
    mov ah, 0x41
    mov bx, 0x55aa
    # dl contains drive number
    int 0x13
    jc no_int13h_extensions

# 读取 余下的 booloader 段
load_rest_of_bootloader_from_disk:
    lea eax, _rest_of_bootloader_start_addr

    # 开始地址 存到一个地方
    mov [dap_buffer_addr], ax

    # 计算磁盘一共几个块 存到一个地方
    lea ebx, _rest_of_bootloader_end_addr
    sub ebx, eax # end - start
    shr ebx, 9 # divide by 512 (block size)
    mov [dap_blocks], bx

    # 计算开始(_start)到结束一共多少个块、存其来
    lea ebx, _start
    sub eax, ebx
    shr eax, 9 # divide by 512 (block size)
    mov [dap_start_lba], eax

    # 使用 int 13 0x42号 功能 ： 从磁盘读 sectors 到 buffer 中
    # 入口参数：ah = 0x42, dl = 磁盘号（0x80 为硬盘）， ds:si = buffer
    lea si, dap
    mov ah, 0x42
    int 0x13
    jc rest_of_bootloader_load_failed

jump_to_second_stage:
    lea eax, [stage_2]
    jmp eax


spin:
    jmp spin

real_mode_println:
    call real_mode_print
    mov al, 13 # \r
    call real_mode_print_char
    mov al, 10 # \n
    jmp real_mode_print_char

real_mode_print:
    cld

real_mode_print_loop:
    lodsb al, BYTE PTR [si]
    test al, al
    jz real_mode_print_done
    call real_mode_print_char
    jmp real_mode_print_loop

real_mode_print_done:
    ret

real_mode_print_char:
    mov ah, 0x0e
    int 0x10
    ret


real_mode_error:
    call real_mode_println
    jmp spin

no_int13h_extensions:
    lea si, no_int13h_extensions_str
    jmp real_mode_error

rest_of_bootloader_load_failed:
    lea si, rest_of_bootloader_load_failed_str
    jmp real_mode_error

debug:
    mov bx, 0x0f01         
    mov eax, 0xb8f01
    mov word ptr ds:[eax], bx
    ret


boot_start_str: .asciz "Booting (first stage)..."
error_str: .asciz "Error: "
no_int13h_extensions_str: .asciz "No support for int13h extensions"
rest_of_bootloader_load_failed_str: .asciz "Failed to load rest of bootloader"

gdt32info:
   .word gdt32_end - gdt32 - 1  # last byte in table
   .word gdt32                  # start of table

gdt32:
    # entry 0 is always unused
    .quad 0
codedesc:
    .byte 0xff
    .byte 0xff
    .byte 0
    .byte 0
    .byte 0
    .byte 0x9a
    .byte 0xcf
    .byte 0
datadesc:
    .byte 0xff
    .byte 0xff
    .byte 0
    .byte 0
    .byte 0
    .byte 0x92
    .byte 0xcf
    .byte 0
gdt32_end:

dap: # disk access packet 4K
    .byte 0x10 # size of dap
    .byte 0 # unused
dap_blocks:
    .word 0 # number of sectors
dap_buffer_addr:
    .word 0 # offset to memory buffer
dap_buffer_seg:
    .word 0 # segment of memory buffer
dap_start_lba:
    .quad 0 # start logical block address

.org 510
.word 0xaa55