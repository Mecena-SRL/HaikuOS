#!/usr/bin/env bash
# Build dell'app UEFI, ESP FAT, avvio QEMU virt con firmware QEMU_EFI e ramfb; controlla seriale e pixel accento.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo build --release || exit 1
EFI=target/aarch64-unknown-uefi/release/haiku-uefi.efi
rm -f esp.img build-uefi.log shot.ppm
mkfs.vfat -C esp.img 8192 >/dev/null
mmd -i esp.img ::/EFI ::/EFI/BOOT
mcopy -i esp.img "$EFI" ::/EFI/BOOT/BOOTAA64.EFI
FW=/usr/share/qemu-efi-aarch64/QEMU_EFI.fd
QMP=/tmp/haikuos-uefi-qmp.sock; rm -f "$QMP"
timeout 60 qemu-system-aarch64 -M virt -cpu cortex-a72 -m 256M -nographic -bios "$FW" \
  -device ramfb -qmp unix:"$QMP",server,nowait \
  -drive if=none,id=esp,format=raw,file=esp.img -device virtio-blk-device,drive=esp > build-uefi.log 2>&1 &
PID=$!
sleep 18
python3 - "$QMP" <<'PY'
import json, socket, sys
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1])
f = s.makefile("rw"); f.readline()
for cmd in ({"execute": "qmp_capabilities"}, {"execute": "screendump", "arguments": {"filename": "shot.ppm"}}):
    f.write(json.dumps(cmd) + "\n"); f.flush(); f.readline()
PY
kill "$PID" 2>/dev/null; wait "$PID" 2>/dev/null
echo "--- seriale ---"; grep -a "HaikuOS\|\[ok\]\|errore" build-uefi.log | head -5
fail=0
for pat in "HaikuOS 0.1 - UEFI ARM64" "GOP [0-9]+x[0-9]+"; do
  if grep -aqE "$pat" build-uefi.log; then echo "OK    $pat"; else echo "FALLITO $pat"; fail=1; fi
done
# Pixel accento #FF8400 nello screenshot (il riquadro centrale)
px=$(python3 -c "
from PIL import Image
im = Image.open('shot.ppm').convert('RGB')
print(sum(1 for p in im.getdata() if p == (0xFF,0x84,0x00)))
" 2>/dev/null || echo 0)
if [ "${px:-0}" -gt 1000 ]; then echo "OK    pixel accento: $px"; else echo "FALLITO pixel accento: ${px:-0}"; fail=1; fi
exit $fail
