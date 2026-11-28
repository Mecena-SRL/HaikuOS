#!/usr/bin/env python3
# Prova del mouse in QEMU (QMP): sposta il puntatore sull'icona FILE, clic, screenshot della finestra.
# Uso: mouse-qemu.py <file.png>. Richiede build/haikuos.img e Pillow.
import json, os, socket, subprocess, sys, time
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))  # x86/
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "build", "mouse.png")
SOCK = os.path.join(ROOT, "build", "qmp-mouse.sock")
if os.path.exists(SOCK):
    os.remove(SOCK)

qemu = subprocess.Popen(["qemu-system-x86_64", "-drive", f"file={ROOT}/build/haikuos.img,format=raw,if=ide,index=0,media=disk",
                         "-m", "128M", "-display", "none", "-vga", "std", "-no-reboot",
                         "-serial", "null", "-qmp", f"unix:{SOCK},server,nowait"])
time.sleep(3)
s = socket.socket(socket.AF_UNIX)
for _ in range(50):
    try:
        s.connect(SOCK); break
    except OSError:
        time.sleep(0.2)
rd = s.makefile("r")
rd.readline()

def cmd(name, **args):
    s.sendall((json.dumps({"execute": name, "arguments": args}) + "\n").encode())
    while True:
        r = json.loads(rd.readline())
        if "return" in r or "error" in r:
            return r

def rel(dx, dy):
    # Pacchetti piccoli e lenti: il kernel legge il PS/2 in polling e perderebbe byte con raffiche.
    while dx or dy:
        sx = max(-20, min(20, dx)); sy = max(-20, min(20, dy))
        ev = []
        if sx: ev.append({"type": "rel", "data": {"axis": "x", "value": sx}})
        if sy: ev.append({"type": "rel", "data": {"axis": "y", "value": sy}})
        cmd("input-send-event", events=ev)
        time.sleep(0.08)
        dx -= sx; dy -= sy
    time.sleep(0.5)

def click():
    cmd("input-send-event", events=[{"type": "btn", "data": {"button": "left", "down": True}}])
    time.sleep(0.3)
    cmd("input-send-event", events=[{"type": "btn", "data": {"button": "left", "down": False}}])
    time.sleep(0.5)

cmd("qmp_capabilities")
time.sleep(1)
# Dal centro (160,120) logico alla piastra dell'icona FILE (cella 8,24 logica, centro 16,32).
# 1 unità QMP = 0,5 px logico: -288 orizzontale, -176 verticale.
rel(-400, -176)
rel(32, 0)
rel(0, 0)
click()
time.sleep(0.5)
ppm = os.path.join(ROOT, "build", "mouse.ppm")
cmd("screendump", filename=ppm)
time.sleep(0.5)
Image.open(ppm).save(OUT)
print("salvato", OUT)
qemu.terminate(); qemu.wait()
