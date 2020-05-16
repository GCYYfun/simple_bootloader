target 			:= x86_64-diyos
boottarget 		:= x86_64-diyos-bootloader
mode 			:= debug
boot			:= target/$(boottarget)/$(mode)/bootloader
kernel			:= target/$(target)/$(mode)/diyos
bootbin			:= target/$(boottarget)/$(mode)/boot.bin
kernelbin		:= target/$(target)/$(mode)/os.bin
osimg			:= target/os.img

objdump := rust-objdump --arch-name=x86-64
objcopy := rust-objcopy --binary-architecture=x86-64

.PHONY: kernel build clean qemu run env

env:
kernel:
	cargo xbuild

boot:
	cargo xbuild --bin bootloader

$(bootbin):
	$(objcopy) $(boot) -S -O binary $@

$(kernelbin):
	kernel
	$(objcopy) $(kernelbin) -S -O binary $@

img:
	dd if=/dev/zero of=$(osimg) count=10000
	dd if=$(bootbin) of=$(osimg) conv=notrunc
	dd if=$(kernelbin) of=$(osimg) seek=1 conv=notrunc

asm:
	$(objdump) -d $(boot) | less


clean:
	cargo clean

qemu: 
	img
	qemu-system-x86_64 -drive format=raw,file=$(osimg)

run: build qemu