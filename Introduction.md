# Bios x86_64 bootloader

## 首先 演示

### 一、进入到bootloader 文件夹 make qemu、展示可以启动 OS

### 二、表述 bootloader 启动 方法

+ 面对机器、希望引导起一个OS 首先要获得一些信息  

    ```
    1、我面对的是什么架构的CPU
    2、我选择使用Bios还是UEFI
    ```
    目前 演示 选用的是 __x86_64__ 架构、__Bios__ 方式的启动方式 （以下默认在这个情况下进行描述）

    CPU 架构类型决定了一些 基于CPU的一些特有的要做的操作

    启动方式 决定了一些 要操作的流程

+ 需要什么  
    
    需要两个文件 一个是OS文件、一个是加载OS文件、并要求他们的格式是二进制形式

+ 怎么启动

    目前的CPU体系和Bios、第一个由人来操作的地址是0x7c00、我们应该把bootloader 也就是启动OS用的软件、放在那里来让他执行、

    自然而言 我们就要从存储设备上 把 这些软件 载入到内存 目标位置

+ 启动 bootloader后要做什么

    主要目的是也把 OS 从 硬盘 中加载到内存中、同时可以设置下当OS获得控制权后所面临的环境

## 描述具体过程 参考 bootloader

### Step 0（前提） 
    有一个已经可以正常编译的内核、把他编译成elf文件
### Step 1 build.rs
    编写build.rs文件、来在bootloader build之前、先做一些准备工作

    在这里 build.rs 负责把Step 0准备好的ELF格式文件、使用objcopy工具、去掉 debug 信息、并重新拷贝成为二进制目标文件（.o 文件）、同时修改 section名字和符号表中字段名字

    最后使用ar工具 把文将变成 .a 的静态库文件

    参与到跟bootloader build 的过程中

### Step 2 linker.ld
    编写链接脚本、指定 数据摆放位置和内存加载地址 如下显示
    定义的一些地址名称 在接下来的汇编里还要用到

    ENTRY(_start)

    SECTIONS {
        . = 0x500;
                                        /* 读取内核用的缓存 */
        _kernel_buffer = .;
        . += 512;
                                        /* 页表 区域*/
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
        _memory_map = .;                /* 内存探测 映射存储地方 */
        . += 0x1000;

        _stack_start = .;
        . = 0x7c00;
        _stack_end = .;

        .bootloader :
        {
            /* 第一个512字节 */
            *(.boot-first-stage)

            /* 剩下的bootloader */
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

### Step 3 asm

这个部分大致分为三个阶段

    1. 主要 
        (1.)初始化堆栈
        (2.)启用A20行
        (3.)从磁盘上加载其余的引导程序
        并跳转到阶段2

    2. 主要 
        (1.)从磁盘加载内核
        (2.)创建e820内存映射
        (3.)进入保护模式
        跳转到阶段3

    3. 主要 
        (1.)设置初始页表映射（对等映射bootloader，递归映射P4，将内核blob映射到4MB）
        (2.)启用分页
        (3.)切换到长模式
        跳转到阶段4。

### Step 4 main.rs

先进入 阶段4 这里只整理了一下 必须的变量信息、然后调用了 bootloader main  
    
    #[no_mangle]
    pub unsafe extern "C" fn stage_4() -> ! {
        // 设置一下栈段
        llvm_asm!("mov bx, 0x0 
                mov ss, bx" ::: "bx" : "intel");

        // 获取基本的信息
        let kernel_start = 0x400000;
        let kernel_size = &_kernel_size as *const _ as u64;
        let memory_map_addr = &_memory_map as *const _ as u64;
        let memory_map_entry_count = (mmap_ent & 0xff) as u64; // Extract lower 8 bits
        let page_table_start = &__page_table_start as *const _ as u64;
        let page_table_end = &__page_table_end as *const _ as u64;
        let bootloader_start = &__bootloader_start as *const _ as u64;
        let bootloader_end = &__bootloader_end as *const _ as u64;
        let p4_physical = &_p4 as *const _ as u64;

        // 调用 bootloader main
        bootloader_main(
            IdentityMappedAddr(PhysAddr::new(kernel_start)),
            kernel_size,
            VirtAddr::new(memory_map_addr),
            memory_map_entry_count,
            PhysAddr::new(page_table_start),
            PhysAddr::new(page_table_end),
            PhysAddr::new(bootloader_start),
            PhysAddr::new(bootloader_end),
            PhysAddr::new(p4_physical),
        )
    }


然后在bootloader main 里进行 一些 前期准备 工作 例如页表 映射之类的 把一些信息打包信息传给内核  

```
主要：
    1、读取 kernel 地址 ,用elf 文件解析,获得kernel的 entry 函数

    2、对已使用的虚拟地址、物理页帧作标记，同时也对未使用的地址区域作标记、这些工作 全是在为boot_info作准备、这个是要传给内核的、通过内存方式、

    3、最后 把 刚才准备好的信息比如内核入口地址 boot info 信息 通过汇编的调用、比如call entry地址执行、控制权传给内核，参数应该使用栈的方式固定在其后的地址空间、
```