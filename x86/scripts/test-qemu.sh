#!/usr/bin/env bash
# Avvia HaikuOS in QEMU headless, salva seriale e screenshot, verifica le righe attese.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
rm -f build/serial.log build/screen.ppm build/qmp.sock

timeout 20 qemu-system-x86_64 \
  -drive file=build/haikuos.img,format=raw,if=ide,index=0,media=disk \
  -m 128M -display none -vga std -no-reboot \
  -serial file:build/serial.log \
  -qmp unix:build/qmp.sock,server,nowait &
QEMU_PID=$!

sleep 4
python3 - <<'EOF'
import json, socket, time
s = socket.socket(socket.AF_UNIX); s.connect("build/qmp.sock")
f = s.makefile("rw")
f.readline()  # saluto QMP
for cmd in ({"execute": "qmp_capabilities"},
            {"execute": "screendump", "arguments": {"filename": "build/screen.ppm"}}):
    f.write(json.dumps(cmd) + "\n"); f.flush(); f.readline()
EOF
kill "$QEMU_PID" 2>/dev/null
wait "$QEMU_PID" 2>/dev/null

echo "--- seriale ---"
cat build/serial.log
echo "--- verifica ---"
fail=0
for pat in "HaikuOS 0.1" "firma 0x55AA presente" "file system HAIKUFS1 letto" "Benvenuto in HaikuOS"; do
  if grep -q "$pat" build/serial.log; then echo "OK    $pat"; else echo "FALLITO $pat"; fail=1; fi
done
exit $fail
