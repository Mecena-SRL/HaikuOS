#!/usr/bin/env bash
# Build dell'app UEFI, ESP FAT, avvio QEMU virt con firmware QEMU_EFI e ramfb.
# Input: tasti dal seriale (dd Invio = apre CALC, x chiude, q esce). Controlla seriale, icone e schermo.
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
( sleep 7; printf 'sdd\r'; sleep 2; printf '\r'; sleep 1; printf '\r'; sleep 2; printf 'x'; sleep 2; printf 'q'; sleep 4 ) | \
timeout 60 qemu-system-aarch64 -M virt -cpu cortex-a72 -m 256M -nographic -bios "$FW" \
  -device ramfb -qmp unix:"$QMP",server,nowait \
  -drive if=none,id=esp,format=raw,file=esp.img -device virtio-blk-device,drive=esp > build-uefi.log 2>&1 &
sleep 11
python3 - "$QMP" <<'PY'
import json, socket, sys
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1])
f = s.makefile("rw"); f.readline()
for cmd in ({"execute": "qmp_capabilities"}, {"execute": "screendump", "arguments": {"filename": "shot.ppm"}}):
    f.write(json.dumps(cmd) + "\n"); f.flush(); f.readline()
PY
wait
echo "--- seriale ---"; grep -a "HaikuOS\|\[ok\]\|errore" build-uefi.log | head -12
fail=0
for pat in "HaikuOS 0.1 - UEFI ARM64" "desktop GOP [0-9]+x[0-9]+ [0-9]+ bit" "app aperta: SET" "SET: TEMA 1" "SET: TEMA 2" "app chiusa" "uscita"; do
  if grep -aqE "$pat" build-uefi.log; then echo "OK    $pat"; else echo "FALLITO $pat"; fail=1; fi
done
# Ogni icona (80x80 a 800x600) ha 6400 pixel del suo colore; lo screenshot è a finestra aperta.
python3 - <<'PY' || fail=1
from PIL import Image
from collections import Counter
icons = [0xE8A33D,0x4FC3C1,0xE07A5F,0x8E7DBE,0xF2CC8F,0x81B29A,0xEF476F,0x06D6A0,0xFFD166]
im = Image.open('shot.ppm').convert('RGB')
cnt = Counter(im.getdata())
ok = True
for c in icons:
    n = cnt[((c >> 16) & 0xFF, (c >> 8) & 0xFF, c & 0xFF)]
    ok = ok and n == 6400
print(("OK    " if ok else "FALLITO ") + "9 icone a 6400 pixel")
raise SystemExit(0 if ok else 1)
PY
exit $fail
