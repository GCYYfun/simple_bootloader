#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(llvm_asm)]
use core::panic::PanicInfo;


global_asm!(include_str!("boot/boot.asm"));

const SECTSIZE:u32 = 512;
const SCRATCH:u32 = 0x10000;
pub const ELF_MAGIC : [u8; 4] = [0x7f, b'E', b'L', b'F'];

#[no_mangle]
pub unsafe extern "C" fn bmain() ->!{

    let mut  scratch_space = SCRATCH ;

    readseg(&mut scratch_space, SECTSIZE*8, 0);

    let elf: *mut ElfHeader = scratch_space as *mut ElfHeader;

    let a = ELF_MAGIC;

    if (*elf).magic != ELF_MAGIC {
        loop{};
    }
    // 读取 文件 完毕 


    // 加载 programe segment
    let mut ph    = (elf as u64 + (*elf).phoff) as  *mut ProgramHeader;
    let phentsize = (*elf).phentsize as u64;
    let end       = ph as u16 + (*elf).phnum;
    
    while (ph as u64) < (end as u64) {
        readseg(&mut ((*ph).paddr as u32),((*ph).memsz) as u32,((*ph).offset) as u32);
        ph = (ph as u64 + phentsize) as *mut ProgramHeader;
    }

    let f = (*elf).entry as *const fn();

    (*f)();

    extern "C" {
        fn multiboot_info();
    }

    loop{}
}


#[panic_handler]
#[no_mangle]
pub fn panic(_info: &PanicInfo) -> ! {
    loop {}
}


// ELF 文件 结构

#[repr(C)]
pub struct ElfHeader {
    magic: [u8; 4],
    class: u8,
    endianness: u8,
    header_version: u8,
    abi: u8,
    abi_version: u8,
    unused: [u8; 7],
    elftype: u16,
    machine: u16,
    elf_version: u32,
    entry: u64,
    phoff: u64,
    shoff: u64,
    flags: u32,
    ehsize: u16,
    phentsize: u16,
    phnum: u16,
    shentsize: u16,
    shnum: u16,
    shstrndx: u16,
}

#[derive(Debug)]
#[repr(C)]
pub struct ProgramHeader {
    p_type:   u32,
    flags:  u32,
    offset: u64,
    vaddr:  u64,
    paddr:  u64,
    filesz: u64,
    memsz:  u64,
    align:  u64,
}

#[derive(Debug)]
#[repr(C)]
pub struct SectionHeader{
    name: u32,
    sh_type: u32,
    flags: u64,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addralign: u64,
    entsize: u64,
}

// 读取 段
// 语义：在什么地方、读取多少字节数据到 哪个物理地址
unsafe fn readseg(pa:&mut u32 ,count:u32,offset:u32) {
    // 接收到 一个物理地址 作用是把数据放入的位置 
    // 一个 数量 来规定 读取 数量
    // 一个 偏移 来规定读取位置

    // let mut pa_porxy = pa as u32;
    // assert_eq!(pa,pa_porxy);
    // 计算 一个结束位置
    let end = *pa + count;
    // 对齐 向下对齐物理地址
    *pa = *pa & !(SECTSIZE -1);
    // 计算 块 偏移位置
    let mut offset = (offset / SECTSIZE) + 1;

    while *pa < end {
        // 读取 信息到pa
        readsect(pa, offset);
        // 更新 位置 
        *pa = *pa  + SECTSIZE;
        // 向后偏移一块
        offset+=1;
    }

}

unsafe fn readsect(dst:&mut u32, offset:u32)
{

    waitdisk();

    outb(0x1F2, 1);		
    outb(0x1F3, offset as u8);
    outb(0x1F4, (offset >> 8) as u8);
    outb(0x1F5, (offset >> 16) as u8);
    outb(0x1F6, ((offset >> 24) | 0xE0)as u8);
    outb(0x1F7, 0x20);	// cmd 0x20 - read sectors

    waitdisk();

    insl(0x1F0,dst,SECTSIZE/4);

}

unsafe fn waitdisk() {
    while inb(0x1F7) & 0xC0 != 0x40 {}
}

pub unsafe fn inb(port: u16) -> u8 {
    let data: u8;
    llvm_asm!("inb %dx, %al" : "={al}"(data) : "{dx}"(port) :: "volatile");
    data
}

#[inline]
unsafe fn outb(port:u32,data:u8){
    llvm_asm!("outb %al,%dx"::"{al}"(data),"{dx}"(port)::"volatile");
}
#[inline]
unsafe fn insl(port:u32,mut addr:&mut u32,mut cnt:u32){
    llvm_asm!(" cld\n\trepne\n\tinsl"
    :"={si}" (addr), "={cx}" (cnt)
    :"{dx}" (port), "{si}" (addr), "{cx}" (cnt)
    :"memory", "cc");
}
