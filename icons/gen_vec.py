#!/usr/bin/env python3
"""Icone vettoriali per Canvas::icon_vec: rettangoli (x, y, w, h) su griglia 16x16.

Fonte: ICONS di icons/gen.py (stessi glifi di icons/svg). Ogni icona è rasterizzata
a 16x16 (alfa >= 128) e le righe sono unite in rettangoli.
Output: gui/src/icon_vec_data.rs (generato, non modificare a mano).
Uso: python3 icons/gen_vec.py   (richiede Pillow)  ->  poi python3 icons/gen_vec.py --preview
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import gen  # noqa: E402

OUT = os.path.join(HERE, '..', 'gui', 'src', 'icon_vec_data.rs')
SIZE = 16


def mask_of(prims):
    """Maschera 16x16 monocroma: forme non-NAVY piene, meno i dettagli NAVY (fori).

    gen.render disegna sempre lo sfondo BG: lo spostiamo fuori tela.
    """
    gen.BG = ('r', 1000, 1000, 1, 1, gen.NAVY)
    pos = gen.render([p for p in prims if p[-1] != gen.NAVY], SIZE, ss=8).getchannel('A').load()
    neg = gen.render([p for p in prims if p[-1] == gen.NAVY], SIZE, ss=8).getchannel('A').load()
    return lambda x, y: pos[x, y] >= 128 and neg[x, y] < 128


def rects_of(prims):
    """Maschera 16x16 -> rettangoli, unendo i run orizzontali uguali di righe consecutive."""
    on = mask_of(prims)
    out = []
    open_runs = {}  # (x0, x1) -> indice in out, per run che continuano dalla riga prima
    for y in range(SIZE):
        runs, x = [], 0
        while x < SIZE:
            if on(x, y):
                x0 = x
                while x < SIZE and on(x, y):
                    x += 1
                runs.append((x0, x))
            else:
                x += 1
        nxt = {}
        for (x0, x1) in runs:
            if (x0, x1) in open_runs:
                i = open_runs[(x0, x1)]
                rx, ry, rw, rh = out[i]
                out[i] = (rx, ry, rw, rh + 1)
                nxt[(x0, x1)] = i
            else:
                out.append((x0, y, x1 - x0, 1))
                nxt[(x0, x1)] = len(out) - 1
        open_runs = nxt
    return out


def build():
    return [(name, rects_of(prims)) for name, prims in gen.ICONS.items()]


def write_rust(table):
    lines = [
        '//! GENERATO da icons/gen_vec.py: non modificare a mano.',
        '//! Icone 16x16 per `Canvas::icon_vec`: rettangoli (x, y, w, h), in ordine di gen.py.',
        '',
        'pub const ICON_VEC: &[(&str, &[(i32, i32, i32, i32)])] = &[',
    ]
    for name, rects in table:
        body = ', '.join(f'({x}, {y}, {w}, {h})' for x, y, w, h in rects)
        lines.append(f'    ("{name}", &[{body}]),')
    lines.append('];')
    with open(OUT, 'w') as f:
        f.write('\n'.join(lines) + '\n')


def preview(table):
    from PIL import Image, ImageDraw, ImageFont
    W, H, CW, CH, S = 1920, 1080, 192, 150, 6
    cols = W // CW
    sheet = Image.new('RGB', (W, H), gen.NAVY)
    d = ImageDraw.Draw(sheet)
    font = ImageFont.load_default()
    for i, (name, rects) in enumerate(table):
        cx, cy = (i % cols) * CW, (i // cols) * CH
        ox, oy = cx + (CW - SIZE * S) // 2, cy + 12
        for x, y, w, h in rects:
            d.rectangle([ox + x * S, oy + y * S, ox + (x + w) * S - 1, oy + (y + h) * S - 1], fill=gen.CREAM)
        d.text((cx + 8, cy + 12 + SIZE * S + 14), name, fill=gen.CREAM, font=font)
    sheet.save(os.path.join(HERE, '..', 'design', 'screens', 'icon_vec_1920x1080.png'))


if __name__ == '__main__':
    table = build()
    if '--preview' in sys.argv:
        preview(table)
    else:
        write_rust(table)
        print(f'{len(table)} icone -> {os.path.normpath(OUT)}')
