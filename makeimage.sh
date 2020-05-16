#! /bin/bash

echo make image
dd if=/dev/zero of=target/x86_64-diyos-bootloader/debug/os.img~ bs=512 count=20000 2>/dev/null
dd if=target/x86_64-diyos-bootloader/debug/boot.bin of=target/x86_64-diyos-bootloader/debug/os.img~ conv=notrunc 2>/dev/null
dd if=target/x86_64-diyos-bootloader/debug/os.bin of=target/x86_64-diyos-bootloader/debug/os.img~ seek=1 conv=notrunc 2>/dev/null
mv target/x86_64-diyos-bootloader/debug/os.img~ target/x86_64-diyos-bootloader/debug/os.img
echo make finish