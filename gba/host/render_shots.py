"""Converte i .raw di render_shots.c (VRAM mode 3, RGB555) in PNG 3x con pixel netti.
Uso: python3 host/render_shots.py RAWDIR OUTDIR
"""
import glob
import os
import sys

import numpy as np
from PIL import Image

W, H, SCALE = 240, 160, 3


def rgb555_to_rgb(raw):
    v = np.frombuffer(raw, dtype="<u2").reshape(H, W).astype(np.uint16)
    out = np.zeros((H, W, 3), dtype=np.uint8)
    for i, shift in enumerate((0, 5, 10)):
        c = (v >> shift) & 31
        out[..., i] = (c << 3) | (c >> 2)
    return out


def main(rawdir, outdir):
    os.makedirs(outdir, exist_ok=True)
    for path in sorted(glob.glob(os.path.join(rawdir, "*.raw"))):
        with open(path, "rb") as f:
            img = Image.fromarray(rgb555_to_rgb(f.read()))
        img = img.resize((W * SCALE, H * SCALE), Image.NEAREST)
        name = os.path.splitext(os.path.basename(path))[0] + ".png"
        img.save(os.path.join(outdir, name), optimize=True)
        print(name)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
