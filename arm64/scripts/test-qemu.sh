#!/usr/bin/env bash
# Build ARM64 e avvio headless su QEMU virt: seriale in uscita e un carattere in ingresso (interrupt UART).
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo build --release || exit 1
LOG=build-serial.log
rm -f "$LOG"
(sleep 2; printf 'K') | timeout 6 qemu-system-aarch64 -M virt -cpu cortex-a72 -m 128M -nographic \
  -kernel target/aarch64-unknown-none/release/haiku-arm64 > "$LOG" 2>&1
echo "--- seriale ---"; cat "$LOG"
fail=0
for pat in "HaikuOS 0.1 - kernel ARM64" "EL1" "timer generico: [0-9]* Hz" "MMU attiva" "GIC e UART RX" "RX:K" "timer con interrupt: 3 tick"; do
  if grep -qE "$pat" "$LOG"; then echo "OK    $pat"; else echo "FALLITO $pat"; fail=1; fi
done
exit $fail
