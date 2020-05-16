#! /bin/bash

echo make image
dd if=/dev/zero of=target/os.img~ bs=512 count=20000 2>/dev/null
dd if=target/x86_64-diyos-bootloader/debug/boot.bin of=target/os.img~ conv=notrunc 2>/dev/null
dd if=target/x86_64-diyos/debug/diyos.bin of=target/os.img~ seek=1 conv=notrunc 2>/dev/null
mv target/os.img~ target/os.img
echo make finish