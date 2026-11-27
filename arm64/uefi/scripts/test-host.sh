#!/usr/bin/env bash
# Test della grafica sull'host (pacchetto pixel 16/32 bit e desktop), senza firmware.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$(mktemp -d)"
rustc --edition 2021 --test "$ROOT/src/gfx.rs" -o "$OUT/gfx_test"
"$OUT/gfx_test"
rm -rf "$OUT"
