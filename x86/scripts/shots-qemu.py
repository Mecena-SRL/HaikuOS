#!/usr/bin/env python3
# Avvia HaikuOS in QEMU headless e salva screenshot PNG di alcuni stati (desktop, menu, finestra).
# Uso: shots-qemu.py <cartella_output>. Richiede build/haikuos.img e Pillow.
import json, os, socket, subprocess, sys, time
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))  # x86/
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "build")
os.makedirs(OUT, exist_ok=True)
SOCK = os.path.join(ROOT, "build", "qmp-shots.sock")
if os.path.exists(SOCK):
    os.remove(SOCK)

qemu = subprocess.Popen(["qemu-system-x86_64", "-drive", f"file={ROOT}/build/haikuos.img,format=raw,if=ide,index=0,media=disk",
                         "-m", "128M", "-display", "none", "-vga", "std", "-no-reboot",
                         "-serial", f"file:{ROOT}/build/serial-shots.log", "-qmp", f"unix:{SOCK},server,nowait"])
time.sleep(3)
s = socket.socket(socket.AF_UNIX)
for _ in range(50):
    try:
        s.connect(SOCK); break
    except OSError:
        time.sleep(0.2)
rd = s.makefile("r")
rd.readline()  # saluto QMP

def cmd(name, **args):
    s.sendall((json.dumps({"execute": name, "arguments": args}) + "\n").encode())
    while True:  # salta gli eventi asincroni
        r = json.loads(rd.readline())
        if "return" in r or "error" in r:
            return r

def key(*qcodes):
    cmd("send-key", keys=[{"type": "qcode", "data": q} for q in qcodes])
    time.sleep(0.3)

def shot(name):
    ppm = os.path.join(ROOT, "build", "shot.ppm")
    cmd("screendump", filename=ppm)
    time.sleep(0.5)
    Image.open(ppm).save(os.path.join(OUT, name))
    print("salvato", os.path.join(OUT, name))

cmd("qmp_capabilities")
time.sleep(1)
shot("01_desktop.png")
key("meta_l")                                  # menu applicazioni
for q in ["t", "e", "r", "m"]:
    key(q)                                     # ricerca "term"
shot("02_menu_ricerca.png")
key("ret")                                     # apre il terminale
key("alt")                                     # nessun effetto: controllo che il tasto non rompa nulla
shot("03_finestra_terminale.png")
qemu.terminate(); qemu.wait()
