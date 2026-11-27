#!/usr/bin/env bash
# Build ARM64 e avvio headless su QEMU virt: controlla il seriale PL011.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo build --release || exit 1
LOG=build-serial.log
rm -f "$LOG"
timeout 10 qemu-system-aarch64 -M virt -cpu cortex-a72 -m 128M -nographic \
  -kernel target/aarch64-unknown-none/release/haiku-arm64 -serial file:"$LOG" >/dev/null 2>&1
echo "--- seriale ---"; cat "$LOG"
fail=0
for pat in "HaikuOS 0.1 - kernel ARM64" "EL1"; do
  if grep -q "$pat" "$LOG"; then echo "OK    $pat"; else echo "FALLITO $pat"; fail=1; fi
done
exit $fail
