#!/usr/bin/env bash
# Test sull'host della grafica (formato 16/32 bit, desktop, font) e della calcolatrice, senza firmware.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$(mktemp -d)"
for f in gfx calc; do
  rustc --edition 2021 --test "$ROOT/src/$f.rs" -o "$OUT/${f}_test"
  "$OUT/${f}_test"
done
rm -rf "$OUT"
