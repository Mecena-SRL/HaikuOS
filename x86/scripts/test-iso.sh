#!/usr/bin/env bash
# Avvia build/haikuos.iso in QEMU headless: controlla il seriale e salva build/iso_screen.ppm.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
rm -f build/iso_serial.log build/iso_screen.ppm build/qmp.sock

timeout 25 qemu-system-x86_64 \
  -cdrom build/haikuos.iso -m 256M -vga std -display none -no-reboot \
  -serial file:build/iso_serial.log \
  -qmp unix:build/qmp.sock,server,nowait &
QEMU_PID=$!

sleep 8
python3 - <<'PY'
import json, socket
s = socket.socket(socket.AF_UNIX); s.connect("build/qmp.sock")
f = s.makefile("rw"); f.readline()
for cmd in ({"execute": "qmp_capabilities"},
            {"execute": "screendump", "arguments": {"filename": "build/iso_screen.ppm"}}):
    f.write(json.dumps(cmd) + "\n"); f.flush(); f.readline()
PY
kill "$QEMU_PID" 2>/dev/null
wait "$QEMU_PID" 2>/dev/null

echo "--- seriale ---"
cat build/iso_serial.log
echo "--- verifica ---"
fail=0
for pat in "HaikuOS 0.1" "avviato in long mode" "\[ok\] framebuffer"; do
  if grep -q "$pat" build/iso_serial.log; then echo "OK    $pat"; else echo "FALLITO $pat"; fail=1; fi
done
exit $fail
