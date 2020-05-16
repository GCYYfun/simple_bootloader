// #![no_std]
// #![no_main]

// #![feature(global_asm)]
// #![feature(llvm_asm)]

// use core::panic::PanicInfo;

// use diyos;
// use diyos::{println};

// global_asm!(include_str!("boot/boot.asm"));

// const SECTSIZE:u32 = 512;

// pub const ELF_MAGIC : [u8; 4] = [0x7f, b'E', b'L', b'F'];

// #[repr(C)]
// pub struct ElfHeader {
//     magic: [u8; 4],
//     class: u8,
//     endianness: u8,
//     header_version: u8,
//     abi: u8,
//     abi_version: u8,
//     unused: [u8; 7],
//     elftype: u16,
//     machine: u16,
//     elf_version: u32,
//     entry: u64,
//     phoff: u64,
//     shoff: u64,
//     flags: u32,
//     ehsize: u16,
//     phentsize: u16,
//     phnum: u16,
//     shentsize: u16,
//     shnum: u16,
//     shstrndx: u16,
// }

// #[derive(Debug)]
// #[repr(C)]
// pub struct ProgramHeader {
//     p_type:   u32,
//     flags:  u32,
//     offset: u64,
//     vaddr:  u64,
//     paddr:  u64,
//     filesz: u64,
//     memsz:  u64,
//     align:  u64,
// }

// #[derive(Debug)]
// #[repr(C)]
// pub struct SectionHeader{
//     name: u32,
//     sh_type: u32,
//     flags: u64,
//     addr: u64,
//     offset: u64,
//     size: u64,
//     link: u32,
//     info: u32,
//     addralign: u64,
//     entsize: u64,
// }

// #[no_mangle]
// pub extern "C" fn main() ->!{

//     // 设置 10000 将要放入 elf header 类型的结构体
//     // let elf = 0x10000 as *mut ElfHeader;

//     let elf:*const ElfHeader;

//     // let scratch_space : *const u32 = 0x10000 as *const u32;
//     let scratch_space = 0x10000 as *mut u32;
//     println!("{:?}",scratch_space);
//     // let elf_ptr = elf as *const u32;
//     // 读取 segment 到 elf 这个地质上 读去 4k 大小字节 没有 块偏移
//     unsafe {
//         readseg(scratch_space, 8192, 0);

//         elf = scratch_space as *const ElfHeader;
    
//         let m : [u8; 4] = (*elf).magic;

//         let m2 = ELF_MAGIC;
//         // println!("{}",ELF_MAGIC[0]);
    
//         if (*elf).magic != ELF_MAGIC {
//             panic!("no elf!");
//         }
    
//         // program header 的物理位置 
//         let reelf = elf as u64;
//         let phoff = (*elf).phoff;
    
    
//         let mut ph = (elf as u64 + (*elf).phoff) as *const u32 as *mut ProgramHeader;
//         // end 表示 结束位置 ph 地址 加上 ph的数量   ???
//         let end = ph as u16 + (*elf).phnum;
    
        
//         let _phnum = (*elf).phnum;
    
    
//         let _ph_start = ph as u16;
//     }



//     // 


//     // while (ph as u16) < end {
//     //     readseg((*ph).paddr as *const u32,(*ph).memsz as u32, (*ph).offset as u32);

//     //     ph = (ph as u64 + 1)as *const u32 as *mut ProgramHeader;
//     // }

//     // let f = ((*elf).entry as *const fn()->());

//     // (*f)();

//     loop{}

// }

// #[panic_handler]
// #[no_mangle]
// pub fn panic(_info: &PanicInfo) -> ! {
//     loop {}
// }


// // static ELF:*const u32 = 0;

 

// // 读取 段
// // 语义：在什么地方、读取多少字节数据到 哪个物理地址
// unsafe fn readseg(pa:*const u32 ,count:u32,offset:u32) {
//     // 接收到 一个物理地址 作用是把数据放入的位置 
//     // 一个 数量 来规定 读取 数量
//     // 一个 偏移 来规定读取位置

//     let pa = pa as u32;
//     // 计算 一个结束位置
//     let end = pa + count;
//     // 对齐 向下对齐物理地址
//     let mut pa = pa & !(SECTSIZE -1);
//     // 计算 块 偏移位置
//     let mut offset = (offset / SECTSIZE) + 1;

//     while pa < end {
//         // 读取 信息到pa
//         readsect(&(pa as u8), offset);
//         // 更新 位置 
//         pa = pa  + SECTSIZE;
//         // 向后偏移一块
//         offset+=1;
//     }

// }


// unsafe fn readsect(dst:&u8, offset:u32)
// {

//     waitdisk();

//     outb(0x1F2, 1);		
//     outb(0x1F3, offset as u8);
//     outb(0x1F4, (offset >> 8) as u8);
//     outb(0x1F5, (offset >> 16) as u8);
//     outb(0x1F6, ((offset >> 24) | 0xE0)as u8);
//     outb(0x1F7, 0x20);	// cmd 0x20 - read sectors

//     waitdisk();

//     insl(0x1F0,dst,SECTSIZE/4);

// }

// unsafe fn waitdisk() {
//     while inb(0x1F7) & 0xC0 != 0x40 {}
// }

// pub unsafe fn inb(port: u16) -> u8 {
//     let data: u8;
//     llvm_asm!("inb %dx, %al" : "={al}"(data) : "{dx}"(port) :: "volatile");
//     data
// }

// #[inline]
// unsafe fn outb(port:u32,data:u8){
//     llvm_asm!("outb %al,%dx"::"{al}"(data),"{dx}"(port)::"volatile");
// }

// unsafe fn insl(port:u32,addr:&u8,cnt:u32)->(u8,u32){
//     let mut addr = *addr;
//     let mut cnt = cnt;

//     llvm_asm!(" cld\n\trepne\n\tinsl"
//     :"={si}" (addr), "={cx}" (cnt)
//     :"{dx}" (port), "{si}" (addr), "{cx}" (cnt)
//     :"memory", "cc");

//     (addr,cnt)
// }


#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points



#![feature(global_asm)]
use core::panic::PanicInfo;


global_asm!(include_str!("entry/entry.asm"));

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

static HELLO: &[u8] = b"Hello World!";

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    let vga_buffer = 0xb8000 as *mut u8;

    for (i, &byte) in HELLO.iter().enumerate() {
        unsafe {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }

    loop {}
}
