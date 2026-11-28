#!/usr/bin/env python3
"""Genera le mockup HD di HaikuOS: desktop e finestre in SVG vettoriale, più i PNG del desktop.
Uso: python3 gen.py          SVG + PNG (richiede Playwright con Chromium)
     python3 gen.py --svg    solo SVG
"""
import html
import re
import os
import sys

D = os.path.dirname(os.path.abspath(__file__))
NAVY, BLUE, TERRA = '#14213D', '#3D5A80', '#E07A5F'
SAND, CREAM, SAGE = '#F2CC8F', '#F4EFE6', '#81B29A'
PAPER, GREY, LINE = '#FFFFFF', '#E8E0CC', '#7A7F8C'
SANS = "'DejaVu Sans','Liberation Sans',Arial,sans-serif"
MONO = "'DejaVu Sans Mono','Liberation Mono',monospace"


def esc(s):
    return html.escape(s, quote=True)


def T(x, y, s, size=22, fill=NAVY, anchor='start', mono=False, bold=False):
    wt = ' font-weight="bold"' if bold else ''
    fam = MONO if mono else SANS
    return (f'<text x="{x}" y="{y}" font-size="{size}" fill="{fill}" text-anchor="{anchor}" '
            f'font-family="{fam}"{wt}>{esc(s)}</text>')


def R(x, y, w, h, fill, rx=0, stroke=None):
    st = f' stroke="{stroke}" stroke-width="3"' if stroke else ''
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{fill}"{st}/>'


def C(cx, cy, r, fill):
    return f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="{fill}"/>'


def L(x1, y1, x2, y2, col, sw=3):
    return (f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{col}" '
            f'stroke-width="{sw}" stroke-linecap="round"/>')


def svg(w, h, body):
    grad = (f'<linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">'
            f'<stop offset="0" stop-color="{NAVY}"/><stop offset="1" stop-color="{BLUE}"/></linearGradient>')
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">'
            f'<defs>{grad}</defs>{body}</svg>\n')


_logo = open(os.path.join(D, 'assets', 'logo.svg'), encoding='utf-8').read()
LOGO_INNER = _logo[_logo.index('>') + 1: _logo.rindex('</svg>')]
LOGO_INNER = re.sub(r'<metadata.*?</metadata>', '', LOGO_INNER, flags=re.S)  # manifest C2PA, namespace non definito


def logo(x, y, s):
    return f'<svg x="{x}" y="{y}" width="{s}" height="{s}" viewBox="0 0 2048 2048">{LOGO_INNER}</svg>'


# Icone 96x96 (sfondo a parte)
GLYPH = {
    'files': R(18, 30, 60, 46, SAND, 6) + R(18, 22, 26, 14, SAND, 4),
    'term': R(14, 20, 68, 56, NAVY, 8) + T(22, 60, '>_', 28, SAND, mono=True),
    'calc': R(24, 16, 48, 64, BLUE, 6) + R(31, 24, 34, 12, CREAM, 3)
            + ''.join(R(31 + c * 12, 44 + r * 11, 8, 8, CREAM, 2) for r in range(3) for c in range(3)),
    'gear': C(48, 48, 26, NAVY) + C(48, 48, 10, CREAM),
    'clock': C(48, 48, 30, NAVY) + L(48, 48, 48, 28, CREAM, 5) + L(48, 48, 64, 48, CREAM, 5),
    'list': R(22, 18, 52, 62, SAGE, 6)
            + ''.join(R(30, 30 + i * 14, 8, 8, NAVY, 2) + R(42, 33 + i * 14, 24, 5, NAVY, 2) for i in range(3)),
    'web': C(48, 48, 30, BLUE) + L(18, 48, 78, 48, CREAM, 3) + L(48, 18, 48, 78, CREAM, 3),
    'edit': R(20, 16, 56, 66, BLUE, 6) + ''.join(R(30, 30 + i * 14, 36, 5, CREAM, 2) for i in range(4)),
}


def icon(x, y, label, key, bg=CREAM, scale=1):
    body = (R(0, 0, 96, 96, bg, 18) if bg else '') + GLYPH[key]
    out = f'<g transform="translate({x},{y}) scale({scale})">{body}'
    if label:
        out += T(48, 134, label, 22, CREAM, 'middle')
    return out + '</g>'


TB = 56  # altezza barra del titolo


def window(x, y, w, h, title, body):
    s = (f'<rect x="{x + 6}" y="{y + 10}" width="{w}" height="{h}" rx="18" '
         f'fill="#000" fill-opacity="0.35"/>')
    s += R(x, y, w, h, CREAM, 18, stroke=BLUE)
    s += (f'<path d="M{x},{y + TB} V{y + 18} Q{x},{y} {x + 18},{y} H{x + w - 18} '
          f'Q{x + w},{y} {x + w},{y + 18} V{y + TB} Z" fill="{NAVY}"/>')
    s += C(x + 30, y + 28, 9, TERRA) + C(x + 58, y + 28, 9, SAND) + C(x + 86, y + 28, 9, SAGE)
    s += T(x + 110, y + 36, title, 24, CREAM, 'start', bold=True)
    s += body(x, y + TB, w, h - TB)
    return s


# ---------- contenuti delle app (x, y = origine area contenuto) ----------

def b_files(x, y, w, h):
    s = R(x, y, 240, h, GREY)
    for i, n in enumerate(['Home', 'Documenti', 'Immagini', 'Giochi', 'Progetti', 'Cestino']):
        if i == 0:
            s += R(x + 12, y + 18 + i * 58, 216, 46, BLUE, 10)
        s += T(x + 30, y + 50 + i * 58, n, 22, CREAM if i == 0 else NAVY)
    names = ['Lettere.txt', 'Bilancio.ods', 'Foto.png', 'chess.rom',
             'Note.md', 'Mappa.svg', 'Musica.wav', 'Progetti']
    cols = max(1, (w - 280) // 200)
    for i, n in enumerate(names):
        cx = x + 280 + (i % cols) * 200
        cy = y + 40 + (i // cols) * 200
        if cy + 150 > y + h:
            break
        s += R(cx + 30, cy, 110, 110, TERRA if i == 7 else SAND, 16)
        s += T(cx + 85, cy + 140, n, 22, NAVY, 'middle')
    return s


def b_term(x, y, w, h):
    s = R(x, y, w, h, NAVY)
    rows = [('haikuos:~$ ls', SAGE), ('documenti  giochi  progetti  note.md', SAND),
            ('haikuos:~$ calc 12*7', SAGE), ('84', CREAM)]
    for i, (t, c) in enumerate(rows):
        s += T(x + 28, y + 60 + i * 44, t, 26, c, mono=True)
    nxt = y + 60 + len(rows) * 44
    s += T(x + 28, nxt, 'haikuos:~$', 26, SAGE, mono=True)
    s += R(x + 205, nxt - 24, 16, 30, SAGE)
    return s


def b_calc(x, y, w, h):
    s = R(x + 20, y + 20, w - 40, 110, NAVY, 14)
    s += T(x + 40, y + 60, '12 × 7 =', 24, SAND)
    s += T(x + w - 40, y + 112, '84', 64, CREAM, 'end')
    keys = [['C', '±', '%', '÷'], ['7', '8', '9', '×'], ['4', '5', '6', '−'],
            ['1', '2', '3', '+'], ['0', '.', '⌫', '=']]
    bw = (w - 40 - 3 * 12) / 4
    top = y + 150
    bh = (h - 150 - 20 - 4 * 12) / 5
    for r, row in enumerate(keys):
        for c, k in enumerate(row):
            bx = x + 20 + c * (bw + 12)
            by = top + r * (bh + 12)
            fill = TERRA if k in '÷×−+=' else (BLUE if k in 'C±%⌫' else SAND)
            tc = NAVY if fill == SAND else CREAM
            s += R(round(bx, 1), round(by, 1), round(bw, 1), round(bh, 1), fill, 14)
            s += T(round(bx + bw / 2, 1), round(by + bh / 2 + 11, 1), k, 34, tc, 'middle')
    return s


def b_settings(x, y, w, h):
    s = ''
    rows = [('Luminosità', 'slider', 0.7), ('Tema scuro', 'toggle', True), ('Wi-Fi', 'toggle', True),
            ('Suoni', 'toggle', False), ('Lingua', 'value', 'Italiano'), ('Limiti app', 'value', 'Dal monolite')]
    for i, (lab, kind, val) in enumerate(rows):
        ry = y + 24 + i * 84
        s += R(x + 24, ry, w - 48, 68, PAPER, 14)
        s += T(x + 48, ry + 43, lab, 26, NAVY)
        rx = x + w - 72
        if kind == 'toggle':
            s += R(rx - 90, ry + 18, 80, 32, SAGE if val else '#C9BFAA', 16)
            s += C(rx - 90 + (56 if val else 16), ry + 34, 12, CREAM)
        elif kind == 'slider':
            s += R(rx - 300, ry + 31, 240, 6, '#C9BFAA', 3)
            s += R(rx - 300, ry + 31, round(240 * val), 6, TERRA, 3)
            s += C(rx - 300 + round(240 * val), ry + 34, 13, TERRA)
        else:
            s += T(rx, ry + 43, val, 24, BLUE, 'end')
    return s


def b_clock(x, y, w, h):
    import math
    cx, cy = x + w * 0.26, y + h / 2
    r = min(w * 0.2, h * 0.34)
    s = (f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="{r:.1f}" fill="{PAPER}" '
         f'stroke="{NAVY}" stroke-width="8"/>')
    for k in range(12):
        a = math.radians(k * 30)
        s += L(round(cx + (r - 24) * math.sin(a), 1), round(cy - (r - 24) * math.cos(a), 1),
               round(cx + (r - 8) * math.sin(a), 1), round(cy - (r - 8) * math.cos(a), 1), NAVY, 5)

    def hand(deg, length, col, sw):
        a = math.radians(deg)
        return L(round(cx, 1), round(cy, 1), round(cx + length * math.sin(a), 1),
                 round(cy - length * math.cos(a), 1), col, sw)
    s += hand((10 + 10 / 60) * 30, r * 0.5, NAVY, 10) + hand(10 * 6, r * 0.8, NAVY, 6)
    s += hand(50 * 6, r * 0.9, TERRA, 3) + C(round(cx), round(cy), 9, NAVY)
    dx = x + w * 0.68
    s += T(round(dx), round(y + h * 0.45), '10:10', 96, NAVY, 'middle', bold=True)
    s += T(round(dx), round(y + h * 0.45 + 70), 'sabato 28 novembre', 30, BLUE, 'middle')
    return s


def b_rem(x, y, w, h):
    s = T(x + 36, y + 60, 'Promemoria di oggi', 32, NAVY, bold=True)
    items = [('Test HD su hardware reale', False), ('Rivedere le PR dei mockup', True),
             ('Esportare PNG 3840×2160', False), ('Aggiornare il grafo del progetto', False),
             ('Controllare il consumo di RAM', True)]
    for i, (t, done) in enumerate(items):
        iy = y + 100 + i * 72
        s += R(x + 36, iy, 36, 36, PAPER, 8, stroke=BLUE)
        if done:
            s += L(x + 44, iy + 18, x + 52, iy + 27, TERRA, 5) + L(x + 52, iy + 27, x + 68, iy + 9, TERRA, 5)
        s += T(x + 96, iy + 27, t, 26, LINE if done else NAVY)
    return s


def b_browser(x, y, w, h):
    s = R(x, y, w, h, GREY)
    s += T(x + 30, y + 41, '‹   ›   ↻', 26, NAVY)
    s += R(x + 150, y + 12, w - 190, 44, PAPER, 22, stroke=BLUE)
    s += T(x + 176, y + 41, 'haikuos://home', 24, BLUE, mono=True)
    p = y + 70
    s += R(x, p, w, h - 70, PAPER)
    s += T(x + 60, p + 80, 'Benvenuto in HaikuOS', 44, NAVY, bold=True)
    for i, t in enumerate(['Un sistema leggero, modulare e sicuro.',
                           'Parti piccole con confini chiari.',
                           'Dalla Game Boy Advance ai computer moderni.']):
        s += T(x + 60, p + 140 + i * 40, t, 26, NAVY)
    cw = (w - 120 - 2 * 24) / 3
    for i, t in enumerate(['Documenti', 'Giochi', 'Impostazioni']):
        cx = x + 60 + i * (cw + 24)
        s += R(round(cx, 1), p + 280, round(cw, 1), 150, GREY, 16)
        s += T(round(cx + 28, 1), p + 340, t, 26, NAVY, bold=True)
        s += T(round(cx + 28, 1), p + 382, 'Apri', 22, BLUE)
    return s


def b_reader(x, y, w, h):
    s = R(x, y, w, h, GREY)
    s += T(x + w - 40, y + 36, '1 / 3   ·   100%', 22, LINE, 'end')
    px, pw, py = round(x + w * 0.12), round(w * 0.76), y + 70
    s += R(px, py, pw, h - 100, PAPER, 6)
    s += T(px + 40, py + 70, 'Guida rapida', 40, NAVY, bold=True)
    for i, t in enumerate(['HaikuOS è costruito a strati.',
                           'Ogni parte fa poche cose e le fa bene.', '',
                           'Il monolite conta la memoria,',
                           'controlla i permessi e smista',
                           'i messaggi. Non decide priorità.', '',
                           'Gli agenti chiedono quote entro',
                           'limiti fissi. Mai oltre.']):
        s += T(px + 40, py + 130 + i * 38, t, 26, NAVY)
    return s


def b_editor(x, y, w, h):
    s = R(x, y, w, h - 40, PAPER)
    s += R(x, y, 64, h - 40, GREY)
    code = [
        [('fn ', TERRA), ('main', BLUE), ('() -> ! {', NAVY)],
        [('    let mut ', TERRA), ('desk', NAVY), (' = ', NAVY), ('Desktop', BLUE), ('::new();', NAVY)],
        [('    desk.', NAVY), ('open', BLUE), ('(', NAVY), ('"Benvenuto"', SAGE), (');', NAVY)],
        [('    ', NAVY), ('loop', TERRA), (' { desk.', NAVY), ('tick', BLUE), ('(); }', NAVY)],
        [('}', NAVY)],
    ]
    for i, parts in enumerate(code):
        ly = y + 48 + i * 40
        s += T(x + 46, ly, str(i + 1), 22, LINE, 'end', mono=True)
        s += (f'<text x="{x + 88}" y="{ly}" font-family="{MONO}" font-size="24" xml:space="preserve">'
              + ''.join(f'<tspan fill="{c}">{esc(t)}</tspan>' for t, c in parts) + '</text>')
    s += R(x, y + h - 40, w, 40, BLUE)
    s += T(x + 24, y + h - 13, 'main.rs   Rust   riga 3, col 12   UTF-8', 20, CREAM, mono=True)
    return s


# ---------- desktop ----------

def desktop():
    s = '<rect width="1920" height="1080" fill="url(#bg)"/>'
    s += R(0, 0, 1920, 48, NAVY)
    s += logo(10, 6, 36) + T(56, 33, 'HaikuOS', 24, CREAM, bold=True)
    s += T(1900, 33, 'sab 28 nov   12:00', 22, CREAM, 'end')
    for i, (lab, k) in enumerate([('Files', 'files'), ('Terminale', 'term'), ('Calcolatrice', 'calc'),
                                  ('Impostazioni', 'gear'), ('Orologio', 'clock'), ('Promemoria', 'list')]):
        s += icon(60, 80 + i * 160, lab, k)
    s += window(380, 110, 760, 500, 'Files', b_files)
    s += window(1180, 110, 640, 340, 'Terminale', b_term)
    s += window(1180, 480, 300, 450, 'Calcolatrice', b_calc)
    s += R(560, 976, 800, 88, CREAM, 24)
    for i, k in enumerate(['files', 'term', 'calc', 'gear', 'web', 'edit', 'list']):
        s += icon(600 + i * 100, 991, '', k, bg=GREY, scale=0.6)
    return s


APPS = [
    ('file-manager', 'Gestione file', b_files, 1200, 700),
    ('terminal', 'Terminale', b_term, 1000, 600),
    ('calculator', 'Calcolatrice', b_calc, 380, 560),
    ('settings', 'Impostazioni', b_settings, 800, 600),
    ('clock', 'Orologio', b_clock, 800, 520),
    ('reminders', 'Promemoria', b_rem, 640, 560),
    ('browser', 'Browser', b_browser, 1100, 700),
    ('reader', 'Lettore testo', b_reader, 800, 640),
    ('editor', 'Editor', b_editor, 1000, 640),
]


def write_svgs():
    with open(os.path.join(D, 'desktop.svg'), 'w', encoding='utf-8') as f:
        f.write(svg(1920, 1080, desktop()))
    for name, title, fn, w, h in APPS:
        x, y = (1280 - w) // 2, (800 - h) // 2
        body = svg(1280, 800, '<rect width="1280" height="800" fill="url(#bg)"/>' + window(x, y, w, h, title, fn))
        with open(os.path.join(D, name + '.svg'), 'w', encoding='utf-8') as f:
            f.write(body)


def render_pngs():
    from playwright.sync_api import sync_playwright
    src = os.path.join(D, 'desktop.svg')
    with sync_playwright() as p:
        browser = p.chromium.launch(args=['--no-sandbox'])
        for scale, out in [(1, 'desktop_1920x1080.png'), (2, 'desktop_3840x2160.png')]:
            page = browser.new_page(viewport={'width': 1920, 'height': 1080}, device_scale_factor=scale)
            page.goto('file://' + src)
            page.screenshot(path=os.path.join(D, out))
            page.close()
        browser.close()


if __name__ == '__main__':
    write_svgs()
    if '--svg' not in sys.argv:
        render_pngs()
