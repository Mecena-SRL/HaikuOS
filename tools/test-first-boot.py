#!/usr/bin/env python3
"""Avvia target/haiku.iso in QEMU, percorre il primo avvio via seriale e verifica il risultato."""
import subprocess, time

ISO = "target/haiku.iso"
STEPS = [
    ("Hostname: ", "-bad"),                      # rifiutato
    ("Hostname: ", "haiku-box"),
    ("Language (it/en): ", "fr"),                # rifiutato
    ("Language (it/en): ", "it"),
    ("Timezone (es. Europe/Rome): ", "Europe/Rome"),
    ("Admin username: ", "root"),                # riservato
    ("Admin username: ", "ivan"),
    ("Admin password: ", "short"),               # troppo corta
    ("Admin password: ", "correct horse 9"),
    ("Repeat password: ", "altra password 9"),   # non coincide
    ("Admin password: ", "correct horse 9"),
    ("Repeat password: ", "correct horse 9"),
]
PASSWORD = "correct horse 9"
DONE = "Primo avvio completato."

p = subprocess.Popen(
    ["qemu-system-i386", "-cdrom", ISO, "-serial", "stdio", "-display", "none",
     "-no-reboot", "-cpu", "max", "-m", "64"],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)

pending = b""  # testo non ancora consumato
log = ""       # tutto l'output, per le verifiche finali

def read_until(token, timeout=900):
    global pending, log
    end = time.time() + timeout
    while token.encode() not in pending:
        if time.time() > end:
            raise SystemExit(f"timeout aspettando {token!r}\n{log}")
        chunk = p.stdout.read1(4096)
        if not chunk:
            raise SystemExit(f"QEMU terminato\n{log}")
        pending += chunk
        log += chunk.decode(errors="replace")
    pending = pending[pending.index(token.encode()) + len(token):]

read_until("primo avvio")
for prompt, answer in STEPS:
    read_until(prompt)
    p.stdin.write(answer.encode() + b"\r")
    p.stdin.flush()
read_until(DONE)
p.kill()

cfg = log[log.index("--- configurazione ---"):log.index(DONE)]
print(cfg.strip())
for need in ("hostname=haiku-box", "language=it", "timezone=Europe/Rome",
             "admin=ivan", "admin_password=pbkdf2-sha256$600000$"):
    assert need in cfg, f"manca {need}"
assert PASSWORD not in log, "la password compare in chiaro"
print("OK: primo avvio completato in QEMU, password solo come hash")
