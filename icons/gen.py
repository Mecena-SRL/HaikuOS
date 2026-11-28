#!/usr/bin/env python3
"""Icone base di HaikuOS, tre livelli da un'unica fonte (ICONS, griglia 32x32).

  icons/svg/<nome>.svg          vettoriale, per altri computer
  icons/png/<nome>-{128,512}.png dettagliate (supercampionate)
  icons/gba/<nome>-{16,32}.png   semplificate, pixel netti, 4 colori (GBA)
  icons/preview.png             anteprima di tutte le icone

Uso: python3 icons/gen.py   (richiede Pillow)
"""
import os
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
NAVY, BLUE, CORAL, CREAM = "#14213D", "#3D5A80", "#E07A5F", "#F4F1DE"  # palette del logo

# Primitive: ('rr',x,y,w,h,r,c) ('r',x,y,w,h,c) ('e',x,y,w,h,c) ('ring',x,y,w,h,sp,c)
#            ('p',[(x,y)..],c) ('l',x1,y1,x2,y2,sp,c)
BG = ('rr', 1, 1, 30, 30, 6, NAVY)
ICONS = {
    'file-manager': [('rr', 5, 7, 10, 5, 2, CORAL), ('rr', 5, 10, 22, 15, 2, CORAL)],
    'terminal': [('rr', 5, 7, 22, 18, 2, BLUE), ('l', 9, 13, 13, 16, 2, CREAM),
                 ('l', 13, 16, 9, 19, 2, CREAM), ('r', 15, 19, 6, 2, CREAM)],
    'calculator': [('rr', 8, 5, 16, 22, 2, CREAM), ('r', 10, 7, 12, 5, NAVY)] +
                  [('r', x, y, 3, 3, CORAL) for y in (14, 18, 22) for x in (10, 15, 20)],
    'text-reader': [('rr', 6, 6, 20, 21, 2, CREAM)] +
                  [('l', 10, y, 22, y, 2, BLUE) for y in (11, 15, 19)],
    'editor': [('p', [(19, 8), (24, 13), (13, 24), (8, 25), (7, 20)], CORAL),
               ('p', [(7, 20), (8, 25), (13, 24)], CREAM)],
    'settings': [('e', 7, 7, 18, 18, CREAM), ('e', 13, 13, 6, 6, NAVY),
                 ('r', 14, 4, 4, 5, CORAL), ('r', 14, 23, 4, 5, CORAL),
                 ('r', 4, 14, 5, 4, CORAL), ('r', 23, 14, 5, 4, CORAL)],
    'clock': [('e', 6, 6, 20, 20, CREAM), ('l', 16, 16, 16, 9, 2, NAVY),
              ('l', 16, 16, 21, 18, 2, CORAL)],
    'reminders': [('r', 6, 7, 5, 5, CREAM), ('r', 14, 8, 11, 3, CREAM),
                  ('r', 6, 14, 5, 5, CORAL), ('r', 14, 15, 11, 3, CREAM),
                  ('r', 6, 21, 5, 5, CREAM), ('r', 14, 22, 8, 3, CREAM)],
    'browser': [('e', 6, 6, 20, 20, CREAM), ('ring', 11, 6, 10, 20, 2, NAVY),
                ('l', 6, 16, 26, 16, 2, NAVY)],
    'chess': [('r', 9, 6, 4, 5, CREAM), ('r', 14, 6, 4, 5, CREAM), ('r', 19, 6, 4, 5, CREAM),
              ('r', 9, 10, 14, 3, CREAM), ('p', [(11, 13), (21, 13), (19, 24), (13, 24)], CREAM),
              ('r', 8, 24, 16, 3, CORAL)],
    'games': [('rr', 4, 10, 24, 13, 6, CREAM), ('r', 8, 14, 8, 3, NAVY),
              ('r', 11, 11, 3, 9, NAVY), ('e', 19, 14, 3, 3, CORAL), ('e', 22, 17, 3, 3, CORAL)],
}


def num(v):
    return int(v) if float(v).is_integer() else round(v, 2)


def svg(prims):
    out = []
    for p in [BG] + prims:
        k = p[0]
        if k == 'rr':
            _, x, y, w, h, r, c = p
            out.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{c}"/>')
        elif k == 'r':
            _, x, y, w, h, c = p
            out.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{c}"/>')
        elif k == 'e':
            _, x, y, w, h, c = p
            out.append(f'<ellipse cx="{num(x + w / 2)}" cy="{num(y + h / 2)}" rx="{num(w / 2)}" ry="{num(h / 2)}" fill="{c}"/>')
        elif k == 'ring':
            _, x, y, w, h, sp, c = p
            out.append(f'<ellipse cx="{num(x + w / 2)}" cy="{num(y + h / 2)}" rx="{num(w / 2)}" ry="{num(h / 2)}" fill="none" stroke="{c}" stroke-width="{sp}"/>')
        elif k == 'p':
            _, pts, c = p
            out.append(f'<polygon points="{" ".join(f"{x},{y}" for x, y in pts)}" fill="{c}"/>')
        elif k == 'l':
            _, x1, y1, x2, y2, sp, c = p
            out.append(f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{c}" stroke-width="{sp}" stroke-linecap="round"/>')
    return '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">' + ''.join(out) + '</svg>\n'


def render(prims, size, ss=1):
    """Disegna a size px; con ss>1 supercampiona e riduce (antialias)."""
    S = size * ss / 32
    img = Image.new('RGBA', (size * ss, size * ss), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    for p in [BG] + prims:
        k = p[0]
        if k == 'rr':
            _, x, y, w, h, r, c = p
            d.rounded_rectangle([x * S, y * S, (x + w) * S - 1, (y + h) * S - 1], radius=r * S, fill=c)
        elif k == 'r':
            _, x, y, w, h, c = p
            d.rectangle([x * S, y * S, (x + w) * S - 1, (y + h) * S - 1], fill=c)
        elif k == 'e':
            _, x, y, w, h, c = p
            d.ellipse([x * S, y * S, (x + w) * S - 1, (y + h) * S - 1], fill=c)
        elif k == 'ring':
            _, x, y, w, h, sp, c = p
            d.ellipse([x * S, y * S, (x + w) * S - 1, (y + h) * S - 1], outline=c, width=max(1, round(sp * S)))
        elif k == 'p':
            _, pts, c = p
            d.polygon([(x * S, y * S) for x, y in pts], fill=c)
        elif k == 'l':
            _, x1, y1, x2, y2, sp, c = p
            d.line([x1 * S, y1 * S, x2 * S, y2 * S], fill=c, width=max(1, round(sp * S)))
    return img.resize((size, size), Image.LANCZOS) if ss > 1 else img


def main():
    for sub in ('svg', 'png', 'gba'):
        os.makedirs(os.path.join(HERE, sub), exist_ok=True)
    sheet = Image.new('RGBA', (len(ICONS) * 144, 128 + 48 + 24), (255, 255, 255, 255))
    for i, (name, prims) in enumerate(ICONS.items()):
        with open(os.path.join(HERE, 'svg', f'{name}.svg'), 'w') as f:
            f.write(svg(prims))
        for s in (128, 512):
            render(prims, s, ss=4).save(os.path.join(HERE, 'png', f'{name}-{s}.png'))
        for s in (16, 32):
            render(prims, s).save(os.path.join(HERE, 'gba', f'{name}-{s}.png'))
        sheet.paste(render(prims, 128, ss=4), (i * 144, 0), render(prims, 128, ss=4))
        sheet.paste(render(prims, 32), (i * 144 + 48, 136), render(prims, 32))
        sheet.paste(render(prims, 16), (i * 144 + 64, 176), render(prims, 16))
    sheet.save(os.path.join(HERE, 'preview.png'))


if __name__ == '__main__':
    main()
