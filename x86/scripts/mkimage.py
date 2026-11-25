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

text = b"Benvenuto in HaikuOS. Questo file e' stato letto dal disco ATA.\n"
file_lba = FS_LBA + 1
sb = bytearray(SECTOR)
sb[0:8] = b"HAIKUFS1"
struct.pack_into("<I", sb, 8, 1)                       # numero di file
sb[16:16 + 11] = b"LEGGIMI.TXT"                        # entry 0: nome[16]
struct.pack_into("<II", sb, 32, file_lba, len(text))   # lba, dimensione
img[FS_LBA * SECTOR:(FS_LBA + 1) * SECTOR] = sb
img[file_lba * SECTOR:file_lba * SECTOR + len(text)] = text

open(out_path, "wb").write(img)
print(f"immagine: {out_path} ({len(img)} byte, kernel {len(k)} byte)")
