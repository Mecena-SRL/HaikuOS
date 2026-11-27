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
printf 'ciao dal file system\r\n' > leeme.txt
mcopy -i esp.img leeme.txt ::/LEEME.TXT
rm -f leeme.txt
FW=/usr/share/qemu-efi-aarch64/QEMU_EFI.fd
QMP=/tmp/haikuos-uefi-qmp.sock; rm -f "$QMP"
( sleep 7; printf '\r'; sleep 1; printf '\r'; sleep 1; printf 'x'; sleep 1; printf 'sd\r'; sleep 1; printf 'ciao'; sleep 1; printf '\x13'; sleep 1; printf '\x11'; sleep 2; printf 'a'; sleep 1; printf '\r'; sleep 2; printf 'x'; sleep 1; printf 'q'; sleep 4 ) | \
timeout 60 qemu-system-aarch64 -M virt -cpu cortex-a72 -m 256M -nographic -bios "$FW" \
  -device ramfb -netdev user,id=n0 -device virtio-net-device,netdev=n0 -qmp unix:"$QMP",server,nowait \
  -drive if=none,id=esp,format=raw,file=esp.img -device virtio-blk-device,drive=esp > build-uefi.log 2>&1 &
sleep 15
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
for pat in "HaikuOS 0.1 - UEFI ARM64" "desktop GOP [0-9]+x[0-9]+ [0-9]+ bit" "app aperta: FILE" "READ: LEEME.TXT" "app aperta: EDIT" "EDIT: ciao" "app aperta: WEB" "RETE: MAC 52:54:00" "uscita"; do
  if grep -aqE "$pat" build-uefi.log; then echo "OK    $pat"; else echo "FALLITO $pat"; fail=1; fi
done
# Salvataggio EDIT (Ctrl-S): NOTA.TXT deve contenere il testo scritto.
if mtype -i esp.img ::/NOTA.TXT 2>/dev/null | grep -aq "ciao"; then echo "OK    NOTA.TXT salvato"; else echo "FALLITO NOTA.TXT"; fail=1; fi
# Ogni icona (80x80 a 800x600) ha 6400 pixel del suo colore.
python3 - <<'PY' || fail=1
from PIL import Image
from collections import Counter
# Riga superiore (FILE, TERM, CALC) esclusa: il firmware stampa BdsDxe sopra su ramfb.
icons = [0x8E7DBE,0xF2CC8F,0x81B29A,0xEF476F,0x06D6A0,0xFFD166]
im = Image.open('shot.ppm').convert('RGB')
cnt = Counter(im.getdata())
ok = True
for c in icons:
    n = cnt[((c >> 16) & 0xFF, (c >> 8) & 0xFF, c & 0xFF)]
    ok = ok and n == 6400
print(("OK    " if ok else "FALLITO ") + "6 icone a 6400 pixel")
raise SystemExit(0 if ok else 1)
PY
exit $fail
