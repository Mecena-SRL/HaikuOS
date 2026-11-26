#!/usr/bin/env bash
# Crea x86/build/haikuos.iso avviabile da VM (BIOS/UEFI via GRUB, Multiboot2).
# Contenuto: loader Multiboot2 (x86/boot/loader.asm), kernel come modulo, grub.cfg.
# Requisiti: nasm, binutils (ld), grub-mkrescue (grub-pc-bin, xorriso, mtools).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
bash scripts/build.sh                       # kernel -> build/kernel.bin
nasm -f elf32 boot/loader.asm -o build/loader.o
ld -m elf_i386 -Ttext 0x200000 -e _start -o build/loader.elf build/loader.o
rm -rf build/iso build/haikuos.iso
mkdir -p build/iso/boot/grub
cp build/loader.elf build/kernel.bin build/iso/boot/
cp scripts/grub.cfg build/iso/boot/grub/
grub-mkrescue -o build/haikuos.iso build/iso
rm -rf build/iso
echo "x86/build/haikuos.iso pronto"
