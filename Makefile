target 			:= x86_64-diyos
boottarget 		:= x86_64-bootloader
mode 			:= debug
boot			:= bootloader/target/$(boottarget)/$(mode)/bootloader
kernel			:= target/$(target)/$(mode)/diyos
bootbin			:= bootloader/target/$(boottarget)/$(mode)/bootloader.bin
kernelbin		:= target/$(target)/$(mode)/diyos.bin
osimg			:= target/os.img

objdump := rust-objdump --arch-name=x86-64
objcopy := rust-objcopy --binary-architecture=x86-64

.PHONY: kernel build clean qemu run env

env:
kernel:
	cargo xbuild

boot:
	cd bootloader;cargo xbuild

bootbin: boot
	$(objcopy) $(boot) -S -O binary $(bootbin)

kernelbin: kernel
	$(objcopy) $(kernel) -S -O binary $(kernelbin)

img: bootbin kernelbin
	dd if=/dev/zero of=$(osimg) bs=512 count=10000 2>/dev/null
	dd if=$(bootbin) of=$(osimg) conv=notrunc 2>/dev/null
	dd if=$(kernelbin) of=$(osimg) seek=1 conv=notrunc 2>/dev/null

asm-boot:
	$(objdump) -d $(boot)

asm-kernel:
	$(objdump) -d $(kernel)

clean:
	cargo clean

boot-clean:
	cd bootloader;cargo clean

qemu: img
	qemu-system-x86_64 -drive format=raw,file=$(osimg)

debug: img
	qemu-system-x86_64 -s -S -drive format=raw,file=$(osimg)
	
qemu-boot: bootbin
	qemu-system-x86_64 -drive format=raw,file=$(bootbin)

debug-boot: bootbin
	qemu-system-x86_64 -drive format=raw,file=$(bootbin)

run: qemu