.section .boot, "awx"
.intel_syntax noprefix
.code16

# 该阶段设置目标运行模式，从磁盘加载内核，创建e820内存映射，进入保护模式，跳转到第三阶段

second_stage_start_str: .asciz "Booting (second stage)..."
kernel_load_failed_str: .asciz "Failed to load kernel from disk"

kernel_load_failed:
    lea si, [kernel_load_failed_str]
    call real_mode_println
kernel_load_failed_spin:
    jmp kernel_load_failed_spin

stage_2:
    lea si, [second_stage_start_str]
    call real_mode_println

set_target_operating_mode:
# 有些BIOS假设处理器只能在Legacy模式下运行。我们将目标工作模式更改为 "Long Mode Target Only"，
# 因此固件希望每个CPU都能进入Long Mode一次，然后保持在Long Mode中。这允许固件启用模式指定的优化。
# 我们保存这些标志，因为如果不支持回调的话，CF会被设置（在这种情况下，这是一个NOP
    pushf
    mov ax, 0xec00
    mov bl, 0x2
    int 0x15
    popf

    

load_kernel_from_disk:
    # start of memory buffer
    lea eax, _kernel_buffer
    mov [dap_buffer_addr], ax

    # number of disk blocks to load
    mov word ptr [dap_blocks], 1

    # number of start block
    lea eax, _kernel_start_addr
    lea ebx, _start
    sub eax, ebx
    shr eax, 9 # divide by 512 (block size)
    mov [dap_start_lba], eax

    # destination address  4M
    mov edi, 0x400000

    # block count
    lea ecx, _kernel_size
    add ecx, 511 # align up
    shr ecx, 9

load_next_kernel_block_from_disk:
    # load block from disk

    lea si, dap
    mov ah, 0x42
    int 0x13
    jc kernel_load_failed

    # copy block to 2MiB
    push ecx
    push esi
    mov ecx, 512 / 4
    # 因为我们要把一个字ptr移到32位寄存器esi上，
    # 它是一个32位寄存器
    movzx esi, word ptr [dap_buffer_addr]
    # move from esi to edi ecx times.
    rep movsd [edi], [esi]
    pop esi
    pop ecx


    # next block
    mov eax, [dap_start_lba]
    add eax, 1
    mov [dap_start_lba], eax

    sub ecx, 1
    jnz load_next_kernel_block_from_disk



create_memory_map:
    lea di, es:[_memory_map]
    call do_e820

video_mode_config:
    call config_video_mode

enter_protected_mode_again:
    cli
    lgdt [gdt32info]
    mov eax, cr0
    or al, 1    # set protected mode bit
    mov cr0, eax

    push 0x8
    lea eax, [stage_3]
    push eax
    retf

spin32:
    jmp spin32
