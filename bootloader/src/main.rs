#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(llvm_asm)]
use core::panic::PanicInfo;
include!(concat!(env!("OUT_DIR"), "/bootloader_config.rs"));

global_asm!(include_str!("boot/stage1.asm"));
global_asm!(include_str!("boot/stage2.asm"));
global_asm!(include_str!("boot/e820.asm"));
global_asm!(include_str!("boot/stage3.asm"));

global_asm!(include_str!("boot/vga_text_80x25.s"));


use bootloader::bootinfo::{BootInfo, FrameRange};
use core::convert::TryInto;
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{
    frame::PhysFrameRange, page_table::PageTableEntry, Mapper, Page, PageTable, PageTableFlags,
    PageTableIndex, PhysFrame, RecursivePageTable, Size2MiB, Size4KiB, UnusedPhysFrame,
};
use fixedvec::alloc_stack;
use core::{mem,slice};
use x86_64::instructions::tlb;
use usize_conversions::usize_from;

mod boot_info;
mod frame_allocator;
mod level4_entries;
mod page_table;


pub struct IdentityMappedAddr(PhysAddr);

impl IdentityMappedAddr {
    fn phys(&self) -> PhysAddr {
        self.0
    }
    
    fn virt(&self) -> VirtAddr {
        VirtAddr::new(self.0.as_u64())
    }

    fn as_u64(&self) -> u64 {
        self.0.as_u64()
    }
}

// 从 blog_linker 引进的的
extern "C" {
    static mmap_ent: usize;
    static _memory_map: usize;
    static _kernel_start_addr: usize;
    static _kernel_end_addr: usize;
    static _kernel_size: usize;
    static __page_table_start: usize;
    static __page_table_end: usize;
    static __bootloader_end: usize;
    static __bootloader_start: usize;
    static _p4: usize;
}

#[no_mangle]
pub unsafe extern "C" fn stage_4() -> ! {
    // Set stack segment
    llvm_asm!("mov bx, 0x0
          mov ss, bx" ::: "bx" : "intel");

    let kernel_start = 0x400000;
    let kernel_size = &_kernel_size as *const _ as u64;
    let memory_map_addr = &_memory_map as *const _ as u64;
    let memory_map_entry_count = (mmap_ent & 0xff) as u64; // Extract lower 8 bits
    let page_table_start = &__page_table_start as *const _ as u64;
    let page_table_end = &__page_table_end as *const _ as u64;
    let bootloader_start = &__bootloader_start as *const _ as u64;
    let bootloader_end = &__bootloader_end as *const _ as u64;
    let p4_physical = &_p4 as *const _ as u64;

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

fn bootloader_main(
    kernel_start: IdentityMappedAddr,
    kernel_size: u64,
    memory_map_addr: VirtAddr,
    memory_map_entry_count: u64,
    page_table_start: PhysAddr,
    page_table_end: PhysAddr,
    bootloader_start: PhysAddr,
    bootloader_end: PhysAddr,
    p4_physical: PhysAddr,
) -> ! {
    use bootloader::bootinfo::{MemoryRegion, MemoryRegionType};
    use fixedvec::FixedVec;
    use xmas_elf::program::{ProgramHeader, ProgramHeader64};

    // printer::Printer.clear_screen();

    let mut memory_map = boot_info::create_from(memory_map_addr, memory_map_entry_count);

    let max_phys_addr = memory_map
        .iter()
        .map(|r| r.range.end_addr())
        .max()
        .expect("no physical memory regions found");

    // 从 elf 里 读取 需要的信息
    let mut preallocated_space = alloc_stack!([ProgramHeader64; 32]);
    let mut segments = FixedVec::new(&mut preallocated_space);
    let entry_point;
    {
        let kernel_start_ptr = usize_from(kernel_start.as_u64()) as *const u8;
        let kernel = unsafe { slice::from_raw_parts(kernel_start_ptr, usize_from(kernel_size)) };
        let elf_file = xmas_elf::ElfFile::new(kernel).unwrap();
        xmas_elf::header::sanity_check(&elf_file).unwrap();

        entry_point = elf_file.header.pt2.entry_point();

        for program_header in elf_file.program_iter() {
            match program_header {
                ProgramHeader::Ph64(header) => segments
                    .push(*header)
                    .expect("does not support more than 32 program segments"),
                ProgramHeader::Ph32(_) => panic!("does not support 32 bit elf files"),
            }
        }
    }

    // 标记使用的虚拟地址
    let mut level4_entries = level4_entries::UsedLevel4Entries::new(&segments);

    // 在页表中启用对不执行位的支持
    enable_nxe_bit();

    // Create a recursive page table entry
    let recursive_index = PageTableIndex::new(level4_entries.get_free_entry().try_into().unwrap());
    let mut entry = PageTableEntry::new();
    entry.set_addr(
        p4_physical,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    );

    // Write the recursive entry into the page table
    let page_table = unsafe { &mut *(p4_physical.as_u64() as *mut PageTable) };
    page_table[recursive_index] = entry;
    tlb::flush_all();

    let recursive_page_table_addr = Page::from_page_table_indices(
        recursive_index,
        recursive_index,
        recursive_index,
        recursive_index,
    )
    .start_address();
    let page_table = unsafe { &mut *(recursive_page_table_addr.as_mut_ptr()) };
    let mut rec_page_table =
        RecursivePageTable::new(page_table).expect("recursive page table creation failed");

    // 创建一个帧分配器，将已分配的帧标记为内存映射中使用的帧
    let mut frame_allocator = frame_allocator::FrameAllocator {
        memory_map: &mut memory_map,
    };

    // 在帧分配器中标记已使用的内存区域
    {
        let zero_frame: PhysFrame = PhysFrame::from_start_address(PhysAddr::new(0)).unwrap();
        frame_allocator.mark_allocated_region(MemoryRegion {
            range: frame_range(PhysFrame::range(zero_frame, zero_frame + 1)),
            region_type: MemoryRegionType::FrameZero,
        });
        let bootloader_start_frame = PhysFrame::containing_address(bootloader_start);
        let bootloader_end_frame = PhysFrame::containing_address(bootloader_end - 1u64);
        let bootloader_memory_area =
            PhysFrame::range(bootloader_start_frame, bootloader_end_frame + 1);
        frame_allocator.mark_allocated_region(MemoryRegion {
            range: frame_range(bootloader_memory_area),
            region_type: MemoryRegionType::Bootloader,
        });
        let kernel_start_frame = PhysFrame::containing_address(kernel_start.phys());
        let kernel_end_frame =
            PhysFrame::containing_address(kernel_start.phys() + kernel_size - 1u64);
        let kernel_memory_area = PhysFrame::range(kernel_start_frame, kernel_end_frame + 1);
        frame_allocator.mark_allocated_region(MemoryRegion {
            range: frame_range(kernel_memory_area),
            region_type: MemoryRegionType::Kernel,
        });
        let page_table_start_frame = PhysFrame::containing_address(page_table_start);
        let page_table_end_frame = PhysFrame::containing_address(page_table_end - 1u64);
        let page_table_memory_area =
            PhysFrame::range(page_table_start_frame, page_table_end_frame + 1);
        frame_allocator.mark_allocated_region(MemoryRegion {
            range: frame_range(page_table_memory_area),
            region_type: MemoryRegionType::PageTable,
        });
    }

    // 取消ELF文件的映射
    let kernel_start_page: Page<Size2MiB> = Page::containing_address(kernel_start.virt());
    let kernel_end_page: Page<Size2MiB> =
        Page::containing_address(kernel_start.virt() + kernel_size - 1u64);
    for page in Page::range_inclusive(kernel_start_page, kernel_end_page) {
        rec_page_table.unmap(page).expect("dealloc error").1.flush();
    }

    // 为引导信息结构映射一个页面
    let boot_info_page = {
        let page: Page = match BOOT_INFO_ADDRESS {
            Some(addr) => Page::containing_address(VirtAddr::new(addr)),
            None => Page::from_page_table_indices(
                level4_entries.get_free_entry(),
                PageTableIndex::new(0),
                PageTableIndex::new(0),
                PageTableIndex::new(0),
            ),
        };
        let frame = frame_allocator
            .allocate_frame(MemoryRegionType::BootInfo)
            .expect("frame allocation failed");
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            page_table::map_page(
                page,
                frame,
                flags,
                &mut rec_page_table,
                &mut frame_allocator,
            )
        }
        .expect("Mapping of bootinfo page failed")
        .flush();
        page
    };

    // If no kernel stack address is provided, map the kernel stack after the boot info page
    let kernel_stack_address = match KERNEL_STACK_ADDRESS {
        Some(addr) => Page::containing_address(VirtAddr::new(addr)),
        None => boot_info_page + 1,
    };

    // 映射内核段
    let kernel_memory_info = page_table::map_kernel(
        kernel_start.phys(),
        kernel_stack_address,
        KERNEL_STACK_SIZE,
        &segments,
        &mut rec_page_table,
        &mut frame_allocator,
    )
    .expect("kernel mapping failed");

    let physical_memory_offset = if cfg!(feature = "map_physical_memory") {
        let physical_memory_offset = PHYSICAL_MEMORY_OFFSET.unwrap_or_else(|| {
            // 如果不是手动提供的偏移量，请在这里找到一个空闲的P4条目和映射内存
            // 一个4级条目跨越2^48/512字节（超过500gib），所以这个应该足够了
            assert!(max_phys_addr < (1 << 48) / 512);
            Page::from_page_table_indices_1gib(
                level4_entries.get_free_entry(),
                PageTableIndex::new(0),
            )
            .start_address()
            .as_u64()
        });

        let virt_for_phys =
            |phys: PhysAddr| -> VirtAddr { VirtAddr::new(phys.as_u64() + physical_memory_offset) };

        let start_frame = PhysFrame::<Size2MiB>::containing_address(PhysAddr::new(0));
        let end_frame = PhysFrame::<Size2MiB>::containing_address(PhysAddr::new(max_phys_addr));

        for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
            let page = Page::containing_address(virt_for_phys(frame.start_address()));
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
            unsafe {
                page_table::map_page(
                    page,
                    UnusedPhysFrame::new(frame),
                    flags,
                    &mut rec_page_table,
                    &mut frame_allocator,
                )
            }
            .expect("Mapping of bootinfo page failed")
            .flush();
        }

        physical_memory_offset
    } else {
        0 // Value is unused by BootInfo::new, so this doesn't matter
    };

    // 构造 boot info 结构
    let mut boot_info = BootInfo::new(
        memory_map,
        kernel_memory_info.tls_segment,
        recursive_page_table_addr.as_u64(),
        physical_memory_offset,
    );
    boot_info.memory_map.sort();

    //  将引导信息写入引导信息页面
    let boot_info_addr = boot_info_page.start_address();
    unsafe { boot_info_addr.as_mut_ptr::<BootInfo>().write(boot_info) };

    // Make sure that the kernel respects the write-protection bits, even when in ring 0.
    enable_write_protect_bit();

    if cfg!(not(feature = "recursive_page_table")) {
        // unmap recursive entry
        rec_page_table
            .unmap(Page::<Size4KiB>::containing_address(
                recursive_page_table_addr,
            ))
            .expect("error deallocating recursive entry")
            .1
            .flush();
        mem::drop(rec_page_table);
    }

    let entry_point = VirtAddr::new(entry_point);
    unsafe { context_switch(boot_info_addr, entry_point, kernel_memory_info.stack_end) };
}

unsafe fn context_switch(boot_info: VirtAddr, entry_point: VirtAddr, stack_pointer: VirtAddr) -> ! {
    llvm_asm!("call $1; ${:private}.spin.${:uid}: jmp ${:private}.spin.${:uid}" ::
         "{rsp}"(stack_pointer), "r"(entry_point), "{rdi}"(boot_info) :: "intel");
    ::core::hint::unreachable_unchecked()
}

fn enable_nxe_bit() {
    use x86_64::registers::control::{Efer, EferFlags};
    unsafe { Efer::update(|efer| *efer |= EferFlags::NO_EXECUTE_ENABLE) }
}

#[panic_handler]
#[no_mangle]
pub fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

fn enable_write_protect_bit() {
    use x86_64::registers::control::{Cr0, Cr0Flags};
    unsafe { Cr0::update(|cr0| *cr0 |= Cr0Flags::WRITE_PROTECT) };
}


fn phys_frame_range(range: FrameRange) -> PhysFrameRange {
    PhysFrameRange {
        start: PhysFrame::from_start_address(PhysAddr::new(range.start_addr())).unwrap(),
        end: PhysFrame::from_start_address(PhysAddr::new(range.end_addr())).unwrap(),
    }
}

fn frame_range(range: PhysFrameRange) -> FrameRange {
    FrameRange::new(
        range.start.start_address().as_u64(),
        range.end.start_address().as_u64(),
    )
}































































































































































































// const SECTSIZE:u32 = 512;
// const SCRATCH:u32 = 0x10000;
// pub const ELF_MAGIC : [u8; 4] = [0x7f, b'E', b'L', b'F'];

// #[no_mangle]
// pub unsafe extern "C" fn bmain() ->!{

//     let mut  scratch_space = SCRATCH ;

//     readseg(&mut scratch_space, SECTSIZE*8, 0);

//     let elf: *mut ElfHeader = scratch_space as *mut ElfHeader;

//     // let a = ELF_MAGIC;

//     if (*elf).magic != ELF_MAGIC {
//         loop{};
//     }
//     // 读取 文件 完毕 


//     // 加载 programe segment
//     let mut ph    = (elf as u64 + (*elf).phoff) as  *mut ProgramHeader;
//     let phentsize = (*elf).phentsize as u64;
//     let end       = ph as u16 + (*elf).phnum;
    
//     while (ph as u64) < (end as u64) {
//         readseg(&mut ((*ph).paddr as u32),((*ph).memsz) as u32,((*ph).offset) as u32);
//         ph = (ph as u64 + phentsize) as *mut ProgramHeader;
//     }

//     let f = (*elf).entry as *const fn();

//     (*f)();

//     // extern "C" {
//     //     fn multiboot_info();
//     // }

//     loop{}
// }


// #[panic_handler]
// #[no_mangle]
// pub fn panic(_info: &PanicInfo) -> ! {
//     loop {}
// }


// // ELF 文件 结构

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

// // 读取 段
// // 语义：在什么地方、读取多少字节数据到 哪个物理地址
// unsafe fn readseg(pa:&mut u32 ,count:u32,offset:u32) {
//     // 接收到 一个物理地址 作用是把数据放入的位置 
//     // 一个 数量 来规定 读取 数量
//     // 一个 偏移 来规定读取位置

//     // let mut pa_porxy = pa as u32;
//     // assert_eq!(pa,pa_porxy);
//     // 计算 一个结束位置
//     let end = *pa + count;
//     // 对齐 向下对齐物理地址
//     *pa = *pa & !(SECTSIZE -1);
//     // 计算 块 偏移位置
//     let mut offset = (offset / SECTSIZE) + 1;

//     while *pa < end {
//         // 读取 信息到pa
//         readsect(pa, offset);
//         // 更新 位置 
//         *pa = *pa  + SECTSIZE;
//         // 向后偏移一块
//         offset+=1;
//     }

// }

// unsafe fn readsect(dst:&mut u32, offset:u32)
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
// #[inline]
// unsafe fn insl(_port:u32,mut _addr:&mut u32,mut _cnt:u32){
//     // llvm_asm!(" cld\n\trepne\n\tinsl"
//     // :"={si}" (addr), "={cx}" (cnt)
//     // :"{dx}" (port), "{si}" (addr), "{cx}" (cnt)
//     // :"memory", "cc");
// }
