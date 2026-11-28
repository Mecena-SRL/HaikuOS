#!/usr/bin/env python3
"""Icone base di HaikuOS, tre livelli da un'unica fonte (ICONS, griglia 32x32).

  icons/svg/<nome>.svg          vettoriale, per altri computer
  icons/png/<nome>-{128,512}.png dettagliate (supercampionate)
  icons/gba/<nome>-{16,32}.png   semplificate, pixel netti, 4 colori (GBA)
  icons/preview.png             anteprima di tutte le icone

Icone: app (lanciatore), impostazioni, tipi di file, stato, notifiche, cursori.
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
    # --- app del lanciatore (gba/src/desktop.rs, APPS) ---
    'photos': [('rr', 6, 6, 9, 9, 2, CREAM), ('rr', 17, 6, 9, 9, 2, BLUE),
               ('rr', 6, 17, 9, 9, 2, BLUE), ('rr', 17, 17, 9, 9, 2, CORAL)],
    'photo-view': [('rr', 5, 6, 22, 20, 2, CREAM), ('e', 9, 10, 5, 5, CORAL),
                   ('p', [(7, 24), (14, 15), (19, 21), (22, 18), (25, 24)], BLUE)],
    'camera': [('rr', 4, 9, 24, 16, 3, CREAM), ('rr', 11, 6, 10, 4, 1, CREAM),
               ('e', 10, 12, 12, 10, NAVY), ('e', 13, 15, 6, 4, CORAL)],
    'music': [('e', 7, 20, 7, 6, CREAM), ('r', 13, 8, 2, 14, CREAM),
              ('p', [(13, 8), (22, 6), (22, 10), (13, 12)], CORAL)],
    'audio-player': [('e', 5, 5, 22, 22, CREAM), ('p', [(12, 10), (23, 16), (12, 22)], NAVY)],
    'video-player': [('rr', 4, 7, 24, 18, 2, CREAM), ('p', [(13, 11), (21, 16), (13, 21)], CORAL)],
    'maps': [('e', 9, 5, 14, 14, CORAL), ('p', [(9, 15), (23, 15), (16, 27)], CORAL),
             ('e', 13, 9, 6, 6, CREAM)],
    'memo': [('rr', 6, 6, 20, 20, 2, CREAM), ('r', 6, 6, 20, 5, CORAL),
             ('r', 9, 15, 14, 2, BLUE), ('r', 9, 20, 9, 2, BLUE)],
    'passwords': [('e', 5, 11, 10, 10, CREAM), ('e', 8, 14, 4, 4, NAVY),
                  ('r', 14, 15, 13, 3, CREAM), ('r', 22, 18, 3, 4, CREAM)],
    'dictionary': [('rr', 7, 6, 18, 21, 2, CREAM), ('r', 7, 6, 4, 21, CORAL),
                   ('r', 13, 12, 9, 2, BLUE), ('r', 13, 17, 9, 2, BLUE)],
    'calendar': [('rr', 6, 7, 20, 19, 2, CREAM), ('r', 6, 7, 20, 5, CORAL),
                 ('r', 9, 15, 3, 3, BLUE), ('r', 14, 15, 3, 3, BLUE), ('r', 19, 15, 3, 3, BLUE),
                 ('r', 9, 20, 3, 3, BLUE), ('r', 14, 20, 3, 3, CORAL)],
    'messages': [('rr', 5, 7, 22, 15, 6, CREAM), ('p', [(10, 21), (10, 27), (17, 21)], CREAM),
                 ('e', 9, 12, 3, 3, BLUE), ('e', 14, 12, 3, 3, BLUE), ('e', 19, 12, 3, 3, BLUE)],
    'utility': [('ring', 12, 7, 8, 6, 2, CREAM), ('rr', 5, 12, 22, 14, 2, CORAL),
                ('r', 5, 17, 22, 2, NAVY)],
    'software': [('p', [(16, 5), (26, 10), (16, 15), (6, 10)], CREAM),
                 ('p', [(6, 10), (16, 15), (16, 27), (6, 22)], CORAL),
                 ('p', [(16, 15), (26, 10), (26, 22), (16, 27)], BLUE)],
    'help': [('e', 6, 6, 20, 20, CREAM), ('l', 11, 12, 13, 9, 2.5, NAVY),
             ('l', 13, 9, 19, 9, 2.5, NAVY), ('l', 19, 9, 20, 13, 2.5, NAVY),
             ('l', 20, 13, 16, 16, 2.5, NAVY), ('l', 16, 16, 16, 19, 2.5, NAVY),
             ('r', 15, 22, 3, 3, NAVY)],
    'tour': [('r', 9, 6, 2, 21, CREAM), ('p', [(9, 6), (24, 10), (9, 15)], CORAL)],
    'scanner': [('l', 6, 6, 11, 6, 2, CREAM), ('l', 6, 6, 6, 11, 2, CREAM),
                ('l', 21, 6, 26, 6, 2, CREAM), ('l', 26, 6, 26, 11, 2, CREAM),
                ('l', 6, 21, 6, 26, 2, CREAM), ('l', 6, 26, 11, 26, 2, CREAM),
                ('l', 21, 26, 26, 26, 2, CREAM), ('l', 26, 21, 26, 26, 2, CREAM),
                ('r', 9, 15, 14, 2, CORAL)],
    'meteo': [('e', 7, 5, 12, 12, CORAL), ('e', 10, 14, 10, 9, CREAM),
              ('e', 16, 11, 12, 12, CREAM), ('rr', 9, 17, 20, 6, 3, CREAM)],
    'timer': [('rr', 9, 5, 14, 3, 1, CREAM), ('rr', 9, 24, 14, 3, 1, CREAM),
              ('p', [(10, 8), (22, 8), (16, 16)], CORAL), ('p', [(16, 16), (10, 24), (22, 24)], BLUE)],
    'chess-clock': [('e', 5, 8, 11, 14, CREAM), ('e', 16, 8, 11, 14, BLUE),
                    ('r', 14, 4, 4, 3, CORAL)],
    'tally': [('l', 7, 7, 7, 25, 2.5, CREAM), ('l', 11, 7, 11, 25, 2.5, CREAM),
              ('l', 15, 7, 15, 25, 2.5, CREAM), ('l', 19, 7, 19, 25, 2.5, CREAM),
              ('l', 6, 22, 21, 10, 2.5, CORAL)],
    'lorem': [('l', 8, 9, 24, 9, 2.5, CREAM), ('l', 8, 15, 24, 15, 2.5, CREAM),
              ('l', 8, 21, 18, 21, 2.5, CORAL)],
    'shred': [('rr', 5, 6, 22, 8, 2, CORAL), ('r', 9, 15, 2, 11, CREAM),
              ('r', 14, 15, 2, 11, CREAM), ('r', 19, 15, 2, 11, CREAM)],
    # --- impostazioni (gruppi) ---
    'set-theme': [('e', 5, 5, 22, 22, CREAM), ('e', 9, 8, 5, 5, CORAL),
                  ('e', 17, 8, 5, 5, BLUE), ('e', 10, 16, 5, 5, BLUE)],
    'set-display': [('rr', 5, 6, 22, 15, 2, CREAM), ('r', 6, 7, 20, 13, BLUE),
                    ('r', 12, 22, 8, 2, CREAM), ('r', 10, 24, 12, 2, CREAM)],
    'set-sound': [('p', [(5, 12), (10, 12), (16, 7), (16, 25), (10, 20), (5, 20)], CREAM),
                  ('l', 21, 11, 23, 16, 2, CORAL), ('l', 23, 16, 21, 21, 2, CORAL)],
    'set-network': [('l', 16, 8, 8, 22, 2, CREAM), ('l', 16, 8, 24, 22, 2, CREAM),
                    ('l', 8, 22, 24, 22, 2, CREAM), ('e', 12, 4, 8, 8, CORAL),
                    ('e', 4, 18, 8, 8, CREAM), ('e', 20, 18, 8, 8, CREAM)],
    'set-security': [('p', [(16, 5), (26, 9), (24, 18), (16, 27), (8, 18), (6, 9)], CREAM),
                     ('e', 13, 11, 6, 6, NAVY), ('r', 15, 15, 2, 7, NAVY)],
    'set-system': [('rr', 9, 9, 14, 14, 2, CREAM), ('rr', 12, 12, 8, 8, 1, BLUE),
                   ('r', 13, 4, 2, 5, CORAL), ('r', 17, 4, 2, 5, CORAL),
                   ('r', 13, 23, 2, 5, CORAL), ('r', 17, 23, 2, 5, CORAL),
                   ('r', 4, 13, 5, 2, CORAL), ('r', 4, 17, 5, 2, CORAL),
                   ('r', 23, 13, 5, 2, CORAL), ('r', 23, 17, 5, 2, CORAL)],
    # --- tipi di file ---
    'file-folder': [('rr', 4, 8, 11, 5, 2, CORAL), ('rr', 4, 10, 24, 16, 2, CREAM)],
    'file-text': [('rr', 8, 5, 16, 22, 2, CREAM), ('l', 11, 12, 21, 12, 2, BLUE),
                  ('l', 11, 17, 21, 17, 2, BLUE), ('l', 11, 22, 17, 22, 2, BLUE)],
    'file-image': [('rr', 5, 7, 22, 18, 2, CREAM),
                   ('p', [(7, 23), (13, 15), (18, 20), (21, 17), (25, 23)], BLUE),
                   ('e', 19, 10, 4, 4, CORAL)],
    'file-audio': [('r', 6, 13, 3, 6, CREAM), ('r', 11, 9, 3, 14, CORAL),
                   ('r', 16, 5, 3, 22, CREAM), ('r', 21, 10, 3, 12, CORAL)],
    'file-video': [('rr', 5, 8, 22, 16, 2, CREAM), ('p', [(14, 12), (14, 20), (20, 16)], CORAL),
                   ('r', 7, 11, 2, 2, NAVY), ('r', 7, 17, 2, 2, NAVY),
                   ('r', 23, 11, 2, 2, NAVY), ('r', 23, 17, 2, 2, NAVY)],
    'file-archive': [('rr', 7, 7, 18, 19, 2, CREAM), ('r', 14, 7, 4, 4, CORAL),
                     ('r', 14, 13, 4, 4, CORAL), ('r', 14, 19, 4, 4, CORAL)],
    'file-unknown': [('rr', 8, 5, 16, 22, 2, CREAM), ('e', 14, 17, 4, 4, CORAL)],
    # --- stato ---
    'status-battery': [('rr', 4, 10, 22, 12, 2, CREAM), ('r', 26, 14, 2, 4, CREAM),
                       ('r', 7, 13, 12, 6, CORAL)],
    'status-wifi': [('r', 6, 20, 4, 6, CREAM), ('r', 12, 15, 4, 11, CREAM),
                    ('r', 18, 10, 4, 16, CREAM), ('r', 24, 5, 4, 21, CORAL)],
    'status-volume': [('p', [(6, 12), (11, 12), (16, 8), (16, 24), (11, 20), (6, 20)], CREAM),
                      ('l', 21, 12, 21, 20, 2, CORAL)],
    'status-lock': [('rr', 8, 14, 16, 13, 2, CREAM), ('l', 11, 14, 11, 10, 2.5, CREAM),
                    ('l', 11, 10, 21, 10, 2.5, CREAM), ('l', 21, 10, 21, 14, 2.5, CREAM),
                    ('e', 14, 18, 4, 4, NAVY)],
    # --- notifiche ---
    'notify-info': [('e', 5, 5, 22, 22, BLUE), ('r', 15, 13, 3, 9, CREAM), ('e', 15, 8, 3, 3, CREAM)],
    'notify-ok': [('e', 5, 5, 22, 22, CORAL), ('l', 10, 16, 14, 21, 3, CREAM),
                  ('l', 14, 21, 22, 11, 3, CREAM)],
    'notify-warn': [('p', [(16, 5), (27, 25), (5, 25)], CORAL), ('r', 15, 12, 2, 7, NAVY),
                    ('r', 15, 21, 2, 2, NAVY)],
    'notify-error': [('e', 5, 5, 22, 22, CORAL), ('l', 11, 11, 21, 21, 3, CREAM),
                     ('l', 21, 11, 11, 21, 3, CREAM)],
    # --- cursori ---
    'cursor-arrow': [('p', [(7, 4), (7, 25), (12, 20), (16, 28), (19, 26), (15, 19), (22, 19)], CREAM)],
    'cursor-hand': [('rr', 9, 12, 14, 13, 4, CREAM), ('r', 11, 4, 4, 12, CREAM),
                    ('r', 15, 6, 4, 10, CREAM)],
    'cursor-text': [('r', 8, 6, 16, 3, CREAM), ('r', 8, 23, 16, 3, CREAM), ('r', 14, 6, 4, 21, CREAM)],
    'cursor-wait': [('ring', 6, 6, 20, 20, 3, CREAM), ('l', 16, 16, 16, 10, 2, CORAL),
                    ('l', 16, 16, 21, 19, 2, CORAL)],
    'cursor-cross': [('l', 16, 5, 16, 27, 2.5, CREAM), ('l', 5, 16, 27, 16, 2.5, CREAM),
                     ('e', 12, 12, 8, 8, CORAL)],
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
    cols, rows = 8, -(-len(ICONS) // 8)
    sheet = Image.new('RGBA', (cols * 144, rows * 200), (255, 255, 255, 255))
    for i, (name, prims) in enumerate(ICONS.items()):
        x, y = (i % cols) * 144, (i // cols) * 200
        with open(os.path.join(HERE, 'svg', f'{name}.svg'), 'w') as f:
            f.write(svg(prims))
        for s in (128, 512):
            render(prims, s, ss=4).save(os.path.join(HERE, 'png', f'{name}-{s}.png'))
        for s in (16, 32):
            render(prims, s).save(os.path.join(HERE, 'gba', f'{name}-{s}.png'))
        sheet.paste(render(prims, 128, ss=4), (x, y), render(prims, 128, ss=4))
        sheet.paste(render(prims, 32), (x + 48, y + 136), render(prims, 32))
        sheet.paste(render(prims, 16), (x + 64, y + 176), render(prims, 16))
    sheet.save(os.path.join(HERE, 'preview.png'))


if __name__ == '__main__':
    main()
