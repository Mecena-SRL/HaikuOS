#!/usr/bin/env bash
# Build dell'app UEFI, ESP FAT con BOOTAA64.EFI, avvio su QEMU virt con firmware QEMU_EFI.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo build --release || exit 1
EFI=target/aarch64-unknown-uefi/release/haiku-uefi.efi
rm -f esp.img build-uefi.log
mkfs.vfat -C esp.img 8192 >/dev/null
mmd -i esp.img ::/EFI ::/EFI/BOOT
mcopy -i esp.img "$EFI" ::/EFI/BOOT/BOOTAA64.EFI
FW=/usr/share/qemu-efi-aarch64/QEMU_EFI.fd
timeout 40 qemu-system-aarch64 -M virt -cpu cortex-a72 -m 256M -nographic -bios "$FW" \
  -drive if=none,id=esp,format=raw,file=esp.img -device virtio-blk-device,drive=esp > build-uefi.log 2>&1
echo "--- seriale ---"; grep -a "HaikuOS\|\[ok\]" build-uefi.log | head -5
fail=0
for pat in "HaikuOS 0.1 - UEFI ARM64" "ConOut attivo"; do
  if grep -aq "$pat" build-uefi.log; then echo "OK    $pat"; else echo "FALLITO $pat"; fail=1; fi
done
exit $fail
