#!/usr/bin/env python3
"""Genera 5 wallpaper HaikuOS in SVG vettoriale (1920x1080). Palette del logo.
Uso: python3 gen.py   (poi i PNG 1920x1080 e 3840x2160 si renderizzano dagli SVG con Chromium)
"""
import math
import os
import random

D = os.path.dirname(os.path.abspath(__file__))
NAVY, BLUE, TERRA = '#14213D', '#3D5A80', '#E07A5F'
SAND, CREAM, SAGE = '#F2CC8F', '#F4EFE6', '#81B29A'
W, H = 1920, 1080
MONO = "'DejaVu Sans Mono','Liberation Mono',monospace"


def doc(defs, body):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">'
            f'<defs>{defs}</defs>{body}</svg>\n')


def lin(id, c1, c2, x2=0, y2=1):
    return (f'<linearGradient id="{id}" x1="0" y1="0" x2="{x2}" y2="{y2}">'
            f'<stop offset="0" stop-color="{c1}"/><stop offset="1" stop-color="{c2}"/></linearGradient>')


def poly(pts, fill, op=1):
    d = 'M' + ' L'.join(f'{x:.0f},{y:.0f}' for x, y in pts) + ' Z'
    return f'<path d="{d}" fill="{fill}" fill-opacity="{op}"/>'


# 1. NERD: griglia, circuiti e prompt del terminale
def nerd():
    rnd = random.Random(7)
    s = ''
    for x in range(0, W, 60):
        s += f'<line x1="{x}" y1="0" x2="{x}" y2="{H}" stroke="{BLUE}" stroke-opacity=".35"/>'
    for y in range(0, H, 60):
        s += f'<line x1="0" y1="{y}" x2="{W}" y2="{y}" stroke="{BLUE}" stroke-opacity=".35"/>'
    for _ in range(45):  # tracce di circuito, lato destro e basso
        x, y = rnd.randrange(480, W, 60), rnd.randrange(0, H, 60)
        pts = [(x, y)]
        for _ in range(rnd.randint(2, 5)):
            if len(pts) % 2:
                x += rnd.choice([-1, 1]) * rnd.randrange(60, 300, 60)
            else:
                y += rnd.choice([-1, 1]) * rnd.randrange(60, 240, 60)
            pts.append((min(max(x, 480), W - 60), min(max(y, 0), H - 60)))
        d = 'M' + ' L'.join(f'{a},{b}' for a, b in pts)
        s += f'<path d="{d}" fill="none" stroke="{SAGE}" stroke-width="3" stroke-opacity=".55"/>'
        s += f'<circle cx="{pts[-1][0]}" cy="{pts[-1][1]}" r="7" fill="{SAND}" fill-opacity=".8"/>'
    lines = ['haikuos:~$ ls /giochi', 'chess.rom  tetris.rom  pong.rom',
             'haikuos:~$ cc --versione', 'rustc 1.x  no_std  aarch64',
             'haikuos:~$ monolite --stato', 'memoria ok  permessi ok']
    for i, t in enumerate(lines):
        s += (f'<text x="{560 + (i % 2) * 240}" y="{160 + i * 170}" font-family="{MONO}" '
              f'font-size="30" fill="{SAGE if i % 2 == 0 else SAND}" fill-opacity=".45">{t}</text>')
    s += (f'<rect x="1560" y="820" width="22" height="44" fill="{SAGE}" fill-opacity=".7"/>')
    return doc(lin('bg', NAVY, BLUE), f'<rect width="{W}" height="{H}" fill="url(#bg)"/>' + s)


# 2. NATURALISTICO: tramonto sulle colline, sole a fasce
def naturale():
    s = f'<rect width="{W}" height="{H}" fill="url(#sky)"/>'
    sx, sy, r = 1380, 640, 210
    s += f'<circle cx="{sx}" cy="{sy}" r="{r}" fill="{SAND}"/>'
    for k in range(6):  # fasce orizzontali del sole
        y = sy - 40 + k * 46
        s += f'<rect x="{sx - r - 10}" y="{y}" width="{2 * r + 20}" height="{6 + k * 2}" fill="{TERRA}"/>'
    layers = [(640, 40, 300, BLUE, 0.85, 1.3), (740, 55, 220, SAGE, 0.9, 2.1), (860, 35, 160, NAVY, 1, 3.4)]
    for base, amp, _, col, op, ph in layers:
        pts = [(x, base + amp * math.sin(x / 260 + ph) + 18 * math.sin(x / 90 + ph * 2))
               for x in range(-20, W + 40, 20)]
        pts += [(W + 20, H + 20), (-20, H + 20)]
        s += poly(pts, col, op)
    return doc(lin('sky', NAVY, TERRA, 0, 1), s)


# 3. ARTISTICO: composizione geometrica stile Bauhaus
def artistico():
    s = f'<rect width="{W}" height="{H}" fill="{CREAM}"/>'
    s += f'<rect x="1100" y="0" width="820" height="560" fill="{NAVY}"/>'
    s += f'<circle cx="1450" cy="780" r="270" fill="{TERRA}"/>'
    s += f'<path d="M1700,1080 A260,260 0 0 0 1700,560 Z" fill="{SAND}"/>'
    s += poly([(1100, 1080), (1100, 620), (1400, 1080)], SAGE)
    s += f'<rect x="1100" y="560" width="820" height="10" fill="{BLUE}"/>'
    s += f'<circle cx="1700" cy="260" r="90" fill="{BLUE}"/>'
    s += f'<rect x="0" y="940" width="1100" height="140" fill="{BLUE}"/>'
    s += f'<line x1="0" y1="940" x2="1100" y2="940" stroke="{TERRA}" stroke-width="12"/>'
    return doc('', s)


# 4. CULTURALE: libro aperto e greca (motivo classico)
def cultura():
    s = f'<rect width="{W}" height="{H}" fill="url(#bg)"/>'
    step = 40
    for band_y in (40, H - 80):  # greca in alto e in basso
        pts = []
        for i, x in enumerate(range(0, W + step, step)):
            y = band_y if (i // 2) % 2 == 0 else band_y + step
            pts.append((x, y))
        d = 'M' + ' L'.join(f'{x},{y}' for x, y in pts)
        s += f'<path d="{d}" fill="none" stroke="{SAND}" stroke-width="6" stroke-opacity=".6"/>'
    left = (f'M960,400 Q1120,340 1300,400 L1300,760 Q1120,700 960,760 Z')
    right = (f'M1300,400 Q1480,340 1640,400 L1640,760 Q1480,700 1300,760 Z')
    s += f'<path d="{left}" fill="{CREAM}" fill-opacity=".92"/><path d="{right}" fill="{CREAM}" fill-opacity=".92"/>'
    s += f'<line x1="1300" y1="400" x2="1300" y2="760" stroke="{BLUE}" stroke-width="6"/>'
    for i in range(5):
        y = 470 + i * 56
        s += f'<line x1="1010" y1="{y}" x2="1250" y2="{y - 20}" stroke="{BLUE}" stroke-width="5" stroke-opacity=".6"/>'
        s += f'<line x1="1350" y1="{y - 20}" x2="1590" y2="{y}" stroke="{BLUE}" stroke-width="5" stroke-opacity=".6"/>'
    return doc(lin('bg', NAVY, BLUE, 1, 1), s)


# 5. A SCELTA: vapore, ingranaggi e sbuffi (dal logo)
def vapore():
    cx, cy, r = 1460, 660, 230
    s = f'<rect width="{W}" height="{H}" fill="url(#bg)"/>'
    for k in range(9):  # sbuffi di vapore che salgono
        s += (f'<circle cx="{cx - 120 + k * 35 - 140}" cy="{cy - 200 - k * 70}" r="{40 + k * 14}" '
              f'fill="{CREAM}" fill-opacity="{0.22 - k * 0.018:.3f}"/>')
    teeth = ''.join(f'<rect x="{cx - 34}" y="{cy - r - 40}" width="68" height="80" rx="10" '
                    f'fill="{TERRA}" transform="rotate({a} {cx} {cy})"/>' for a in range(0, 360, 30))
    s += teeth
    s += f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="{TERRA}"/>'
    s += f'<circle cx="{cx}" cy="{cy}" r="{r * 0.5}" fill="{NAVY}"/>'
    s += f'<circle cx="{cx}" cy="{cy}" r="{r * 0.22}" fill="{SAND}"/>'
    s += f'<line x1="0" y1="{cy + r + 60}" x2="{W}" y2="{cy + r + 60}" stroke="{SAGE}" stroke-width="6"/>'
    return doc(lin('bg', NAVY, BLUE, 1, 1), s)


if __name__ == '__main__':
    files = {'1_nerd.svg': nerd(), '2_naturale.svg': naturale(), '3_artistico.svg': artistico(),
             '4_cultura.svg': cultura(), '5_vapore.svg': vapore()}
    for name, content in files.items():
        with open(os.path.join(D, name), 'w', encoding='utf-8') as f:
            f.write(content)
