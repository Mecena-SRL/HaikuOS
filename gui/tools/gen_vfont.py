#!/usr/bin/env python3
"""Genera gui/src/vfont_data.rs dai glifi 5x8 di x86/kernel/src/font.rs.

Ogni pixel acceso diventa parte di un rettangolo (greedy, senza sovrapposizioni),
in unità 0..=64 (cella 8x8). Formato ops: vedi gui/src/vfont.rs.
"""
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[2]
src = (ROOT / "x86/kernel/src/font.rs").read_text()
body = src[src.index("static GLYPHS"):]
rows = re.findall(r"\[((?:0x[0-9A-Fa-f]{2},\s*){7}0x[0-9A-Fa-f]{2})\]", body)
assert len(rows) == 95, len(rows)

out = [
    "//! Dati generati da gui/tools/gen_vfont.py: non modificare a mano.",
    "//! Glifi ASCII 0x20..=0x7E (cella 64 unità = em), formato ops in vfont.rs.",
    "pub static GLYPHS: [&[u8]; 95] = [",
]
for idx, row in enumerate(rows):
    bits = [int(v, 16) for v in re.findall(r"0x[0-9A-Fa-f]{2}", row)]
    lit = [[(bits[r] >> (4 - c)) & 1 for c in range(5)] for r in range(8)]
    done = [[False] * 5 for _ in range(8)]
    ops = []
    for r in range(8):
        for c in range(5):
            if not lit[r][c] or done[r][c]:
                continue
            w = 0
            while c + w < 5 and lit[r][c + w] and not done[r][c + w]:
                w += 1
            h = 1
            while r + h < 8 and all(lit[r + h][c + k] and not done[r + h][c + k] for k in range(w)):
                h += 1
            for dr in range(h):
                for k in range(w):
                    done[r + dr][c + k] = True
            ops += [4, 8 * c, 8 * r, 8 * (c + w), 8 * (r + h)]
    out.append(f"    &[{', '.join(map(str, ops))}], // 0x{0x20 + idx:02X}")
out.append("];")
(ROOT / "gui/src/vfont_data.rs").write_text("\n".join(out) + "\n")
