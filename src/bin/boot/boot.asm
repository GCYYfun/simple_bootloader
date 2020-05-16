
.set PROT_MODE_CSEG, 0x8         # kernel code segment selector
.set PROT_MODE_DSEG, 0x10        # kernel data segment selector
.set CR0_PE_ON,      0x1         # protected mode enable flag

.set multiboot_info, 0x7000
.set e820_map, multiboot_info + 52
.set e820_map4, multiboot_info + 56
	
.set MB_flag, multiboot_info
.set MB_mmap_len, multiboot_info + 44
.set MB_mmap_addr, multiboot_info + 48

.globl multiboot_info

.section .text
.global _start

# BIOS 启动 方式
# 目前状态
# mode : real mode
# cs: 0
# ip: 7c00

# 目的:我是bootloader 要加载内核

# New Update Add Multiboot Specification

_start:
    .code16 # 标注一开始是在16位模式下运行、使用指令时应是16位下的指令
    cli     # 关闭外部中断
    cld     # 清除方向标志位 即方向标志位 的那个bit 为0  控制 si、di 向前移动 即自动递增

    # 设置 重要的段寄存器 
    xorw    %ax,%ax       # xor 异或  w word 16bit 异或清零ax     
    movw    %ax,%ds       # 数据段寄存器 设置为0
    movw    %ax,%es       # 额外段寄存器 设置为0
    movw    %ax,%ss       # 栈段寄存器   设置为0





# 开启 A20
# 因为86结构 第21根地址线默认不开启、是历史原因造成的、
# 而现在地址线比21根要多、所以我们要启用这第21根获得连续的地址

# 思路
# IBM 就想出了一个技巧来保证兼容性。
# 那就是，如果键盘控制器输出端口的第2位是低位，则物理地址的第21位被清零
# 否则，第21位可以正常使用。
# 引导加载器用 I/O 指令控制端口 0x64 和 0x60 上的键盘控制器，
# 使其输出端口的第2位为高位，来使第21位地址正常工作


# 虽然大致知道了A20干什么用的、但是怎么开启完全是copy的、并不知道什么意思、但有时间是可以去查的
seta20.1:
  inb     $0x64,%al               # Wait for not busy
  testb   $0x2,%al
  jnz     seta20.1

  movb    $0xd1,%al               # 0xd1 -> port 0x64
  outb    %al,$0x64

seta20.2:
  inb     $0x64,%al               # Wait for not busy
  testb   $0x2,%al
  jnz     seta20.2

  movb    $0xdf,%al               # 0xdf -> port 0x60
  outb    %al,$0x60

  # get the E820 memory map from the BIOS
do_e820:
  movl $0xe820, %eax
  movl $e820_map4, %edi
  xorl %ebx, %ebx
  movl $0x534D4150, %edx
  movl $24, %ecx
  int $0x15
  jc failed
  cmpl %eax, %edx
  jne failed
  testl %ebx, %ebx
  je failed
  movl $24, %ebp

next_entry:
  #increment di
  movl %ecx, -4(%edi)
  addl $24, %edi
  movl $0xe820, %eax
  movl $24, %ecx
  int $0x15
  jc done
  addl $24, %ebp
  testl %ebx, %ebx
  jne next_entry

done:
  movl %ecx, -4(%edi)
  movw $0x40, (MB_flag) #multiboot info flags
  movl $e820_map, (MB_mmap_addr)
  movl %ebp, (MB_mmap_len)

failed:


# 改变为保护模式

  lgdt    gdtdesc                 # 设置 GDTR
  movl    %cr0, %eax              
  orl     0x1, %eax               # 启用 保护模式
  movl    %eax, %cr0

# 长跳 
  ljmp    $PROT_MODE_CSEG,$protected_mode

# 设置32位情况下的信息
.code32
protected_mode:
  movw    $PROT_MODE_DSEG, %ax               # 设置ds 段选择子
  movw    %ax, %ds                # -> DS: Data Segment
  movw    %ax, %es                # -> ES: Extra Segment
  movw    %ax, %ss                # -> SS: Stack Segment
  movw    %ax, %fs                # -> FS
  movw    %ax, %gs                # -> GS

# 设置 栈位置 
jmain:
  movl    $_start, %esp
  call    bmain

# 防止跳回 无限循环
spin:
  jmp     spin


# 简单设置下GDT
.p2align 2                                # 4字节对齐
gdt:
# null
  .word 0, 0
	.byte 0, 0, 0, 0 
# CS
  .word 0xffff, 0x0
  .byte 0
  .byte 0x9a
  .byte 0xcf
  .byte 0
# DS
  .word 0xffff, 0x0
  .byte 0
  .byte 0x92
  .byte 0xcf
  .byte 0


gdtdesc:
  .word   (gdtdesc - gdt - 1)             # sizeof(gdt) - 1
  .long   gdt                             # address gdt


.org 510
.word 0xaa55 # 魔法数