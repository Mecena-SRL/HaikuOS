#!/usr/bin/env bash
# Compila stage1, stage2 e kernel, poi crea build/haikuos.img.
# Requisiti: rustup target x86_64-unknown-none, nasm, binutils (objcopy), python3.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
mkdir -p build

(cd kernel && cargo build --release)
nasm -f bin boot/stage1.asm -o build/stage1.bin
nasm -f bin boot/stage2.asm -o build/stage2.bin
objcopy -O binary kernel/target/x86_64-unknown-none/release/haiku-kernel build/kernel.bin
python3 scripts/mkimage.py build/stage1.bin build/stage2.bin build/kernel.bin build/haikuos.img
