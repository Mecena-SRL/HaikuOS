# Avvio di HaikuOS su x86-64 (QEMU)

Stato: primo passo verificato in QEMU 8.2 (headless, seriale e VGA). Hardware reale e UEFI: **da verificare**.

## Cosa parte (verificato)

| Fase | Cosa fa | Verifica |
|---|---|---|
| MBR (`x86/boot/stage1.asm`) | Carica lo stage 2 da LBA 1 a 0x7E00 con BIOS int 13h | ok |
| Stage 2 (`x86/boot/stage2.asm`) | Attiva A20, legge il kernel a 0x10000, in modo protetto lo copia a 0x100000, attiva paging (identità 1 GB, pagine da 2 MB) e long mode, salta al kernel | ok |
| Kernel (`kernel/`, Rust `no_std`) | Stack, azzeramento BSS, seriale COM1, VGA testo 80x25 | ok |
| Disco ATA | Legge il settore 0 (PIO, LBA28, canale primario master) e verifica la firma 0x55AA | ok |
| File system di prova `HAIKUFS1` | Legge il superblocco a LBA 2048, elenca i file, stampa `LEGGIMI.TXT` | ok |

Esito atteso sul seriale (usato dal test):

```
HaikuOS 0.1 - kernel x86-64 avviato in long mode
[ok] seriale COM1 e VGA attivi
[ok] disco ATA: settore 0 letto, firma 0x55AA presente
...
Benvenuto in HaikuOS. Questo file e' stato letto dal disco ATA.
[ok] file system HAIKUFS1 letto
Sistema fermo.
```

## Cosa NON c'è ancora

- **Utente amministratore, password, prima configurazione:** da fare. Serve prima un driver tastiera (PS/2) e uno storage per l'hash della password.
- **Interfaccia grafica:** da fare. Serve un framebuffer (VBE/GOP) invece del testo VGA.
- **File system vero:** `HAIKUFS1` è un formato di prova a tabella fissa. Un FS reale (es. FAT) viene dopo.
- **Memoria:** non c'è ancora la mappa di memoria BIOS (E820) né un allocatore. **Da verificare.**
- **UEFI e hardware reale:** non provati. **Da verificare.**
- **Interrupt e tastiera:** non configurati. Il kernel si ferma con `hlt`.

## Come si usa

Requisiti (Debian/Ubuntu):

```sh
apt-get install -y qemu-system-x86 nasm binutils python3
rustup target add x86_64-unknown-none
```

Comandi:

```sh
./x86/scripts/build.sh        # compila stage1, stage2, kernel e crea build/haikuos.img
./x86/scripts/test-qemu.sh    # avvio headless: controlla il seriale e salva build/screen.ppm
./x86/scripts/run-qemu.sh     # avvio con finestra e seriale nel terminale
```

## Layout del disco (`x86/scripts/mkimage.py`)

| LBA | Contenuto |
|---|---|
| 0 | stage 1 (firma 0x55AA) |
| 1-32 | stage 2 (16 KB) |
| 33 | dimensione del kernel (u32) |
| 34.. | kernel, binario piatto caricato a 0x100000 |
| 2048 | superblocco `HAIKUFS1` |
| 2049.. | dati dei file |

## Note tecniche

- Il kernel è un binario piatto: nessun rilocamento, `relocation-model=static`, `panic=abort`, senza SSE (target `x86_64-unknown-none`).
- Il kernel non usa `core::fmt` nel percorso di avvio: stampa con `puts` e `put_dec`. Così il codice di avvio resta piccolo e prevedibile.
- Lo stage 2 non usa più la copia "unreal mode": il kernel viene letto in 0x10000 e copiato in modo protetto.
