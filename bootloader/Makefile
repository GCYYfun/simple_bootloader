build:
	KERNEL=/home/own/RustCode/blog_os/blog_os/target/x86_64-blog_os/debug/blog_os KERNEL_MANIFEST=/home/own/RustCode/blog_os/blog_os/Cargo.toml  cargo xbuild --release --features binary,map_physical_memory

copy:build
	rust-objcopy -I elf64-x86-64 -O binary --binary-architecture=i386:x86-64 target/x86_64-bootloader/release/bootloader target/x86_64-bootloader/release/bootloader.bin

qemu: copy
	qemu-system-x86_64 -drive format=raw,file=target/x86_64-bootloader/release/bootloader.bin

qemu-only: 
	cargo xbuild
	rust-objcopy target/x86_64-bootloader/debug/bootloader -S -O binary target/x86_64-bootloader/debug/bootloader.bin
	qemu-system-x86_64 -drive format=raw,file=target/x86_64-bootloader/debug/bootloader.bin

build-diyos:
	KERNEL=/home/own/RustCode/diyos/target/x86_64-diyos/debug/diyos KERNEL_MANIFEST=/home/own/RustCode/diyos/Cargo.toml  cargo xbuild --release --features binary,map_physical_memory

copy-diyos: build-diyos
	rust-objcopy -I elf64-x86-64 -O binary --binary-architecture=i386:x86-64 target/x86_64-bootloader/release/bootloader target/x86_64-bootloader/release/bootloader.bin

qemu-diyos: copy-diyos
	qemu-system-x86_64 -drive format=raw,file=target/x86_64-bootloader/release/bootloader.bin



clean:
	cargo clean