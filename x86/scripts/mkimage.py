#!/usr/bin/env python3
"""Crea l'immagine disco raw di HaikuOS.

Layout (settori da 512 byte):
  LBA 0       stage 1 (MBR, firma 0x55AA)
  LBA 1-32    stage 2
  LBA 33      intestazione kernel (u32 dimensione in byte)
  LBA 34..    kernel (binario piatto caricato a 0x100000)
  LBA 2048    superblocco HAIKUFS1 (file system minimale di prova)
  LBA 2049..  dati dei file
Uso: mkimage.py stage1.bin stage2.bin kernel.bin out.img
"""
import struct
import sys

SECTOR = 512
IMG_SIZE = 2 * 1024 * 1024
FS_LBA = 2048

s1_path, s2_path, k_path, out_path = sys.argv[1:5]
img = bytearray(IMG_SIZE)

s1 = open(s1_path, "rb").read()
assert len(s1) == 512, "stage1 deve essere 512 byte"
img[0:512] = s1

s2 = open(s2_path, "rb").read()
assert len(s2) <= 32 * SECTOR, "stage2 troppo grande"
img[SECTOR:SECTOR + len(s2)] = s2

k = open(k_path, "rb").read()
assert 34 * SECTOR + len(k) < FS_LBA * SECTOR, "kernel troppo grande"
img[33 * SECTOR:34 * SECTOR] = struct.pack("<I", len(k)).ljust(SECTOR, b"\0")
img[34 * SECTOR:34 * SECTOR + len(k)] = k

import os
# File: LEGGIMI.TXT (test) + ogni file di DISK_DIR (default x86/disk), in ordine di nome.
disk_dir = os.environ.get("DISK_DIR", os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "disk"))
files = [(b"LEGGIMI.TXT", b"Benvenuto in HaikuOS. Questo file e' stato letto dal disco ATA.\n")]
if os.path.isdir(disk_dir):
    for name in sorted(os.listdir(disk_dir)):
        path = os.path.join(disk_dir, name)
        if os.path.isfile(path) and name != "LEGGIMI.TXT" and len(name) <= 15 and name.isascii():
            files.append((name.encode(), open(path, "rb").read()))
assert len(files) <= 15, "troppi file (massimo 15)"

sb = bytearray(SECTOR)
sb[0:8] = b"HAIKUFS1"
struct.pack_into("<I", sb, 8, len(files))              # numero di file
lba = FS_LBA + 1
for i, (name, data) in enumerate(files):
    e = 16 + i * 32                                    # entry: nome[16], lba, dimensione
    sb[e:e + len(name)] = name
    struct.pack_into("<II", sb, e + 16, lba, len(data))
    assert lba * SECTOR + len(data) <= IMG_SIZE, "disco pieno"
    img[lba * SECTOR:lba * SECTOR + len(data)] = data
    lba += (len(data) + SECTOR - 1) // SECTOR
img[FS_LBA * SECTOR:(FS_LBA + 1) * SECTOR] = sb

open(out_path, "wb").write(img)
print(f"immagine: {out_path} ({len(img)} byte, kernel {len(k)} byte)")
