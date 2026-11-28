#!/usr/bin/env bash
# Avvia HaikuOS in QEMU con finestra e seriale nel terminale.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
qemu-system-x86_64 \
  -drive file=build/haikuos.img,format=raw,if=ide,index=0,media=disk \
  -m 128M -device VGA,vgamem_mb=64 -no-reboot -serial stdio
