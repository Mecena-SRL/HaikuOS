#!/bin/sh
# Compila il kernel e crea target/haiku.iso avviabile con GRUB (Multiboot2).
set -e
cd "$(dirname "$0")/.."
cargo build -q -p haiku-kernel --target i686-unknown-linux-gnu --release
rm -rf target/iso
mkdir -p target/iso/boot/grub
cp target/i686-unknown-linux-gnu/release/haiku-kernel target/iso/boot/
cp tools/grub.cfg target/iso/boot/grub/
grub-mkrescue -o target/haiku.iso target/iso 2>/dev/null
echo "target/haiku.iso pronto"
