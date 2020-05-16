# simple_bootloader

目录 
```
Simple-bootloader              
    ├── .cargo              
    |   └── config
    ├── src                 
    │   ├── bin               // boot_loader 在这个文件夹下
    │   │   ├── .cargo
    |   |   |   └── config
    │   │   ├── boot                            
    |   |   |   ├── boot.asm                    // boot汇编
    |   |   |   ├── link.ld                     // boot链接
    |   |   |   ├── print.asm                   // 没用
    |   |   |   └── pure.asm                    // 没用    
    │   │   ├── bootloader.rs                   // bootloader 引入汇编和座elf读取
    │   │   └── x86_64-diyos-bootloader.json       
    │   ├── entry                   // 英语
    |   |   ├── entry.asm
    |   |   └── linker64.ld
    │   ├── main.rs                   // cargo xbuild 不成功
    │   └── vga_buffer.rs             // 没用、用上也不执行
    ├── .gitignore                    
    ├── Cargo.toml                    
    ├── Makefile                      // makefile 
    ├── makeimage.sh                  // 用 dd 制作image (弃用、转移到makefile)
    ├── rust-toolchain                // nightly-2020-04-26
    └── x86_64-diyos.jsom             
 ```

 ## 流程

 1、进入 /simple-bootloader/src/bin 输入以下命令 生成 elf文件
 
> cargo xbuild --bin bootloader

2、 生成的elf 在 /target/x86_64-diyos-bootloader/debug/bootloader

3、 把elf 文件 拷贝成为 bin文件 
> rust-objcopy target/x86_64-diyos-bootloader/debug/bootloader -S -O binary target/x86_64-diyos-bootloader/debug/bootloader.bin

4、qemu加载bin文件、gdb加载elf文件、
> qemu-system-x86_64 -drive format=raw,file=target/x86_64-diyos-bootloader/debug/boot.bin

> gdb target/x86_64-diyos-bootloader/debug/bootloader

5、逐步调试、打印 scratch_space 值不对

6、在根目录下 cargo xbuild 生成  diyos elf文件、位置在 target/x86_64-diyos/debug/diyos

7、同样生成 diyos.bin 使用 rust-objcopy
> rust-objcopy target/x86_64-diyos/debug/diyos -S -O binary target/x86_64-diyos-bootloader/debug/diyos.bin

8、把两个bin文件 用dd 拼成 os.img  
os.img 位置在target/os.img

9、qemu 加载 os.img 还是同样的表现

## make file 

1、make qemu
