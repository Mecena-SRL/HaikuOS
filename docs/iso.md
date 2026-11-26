# ISO avviabile di HaikuOS (x86-64, VM)

Stato: avvio verificato in QEMU 8.2 (BIOS, seriale e screenshot del desktop 1280x800). VirtualBox, VMware e hardware reale: **da verificare**.

## Cosa contiene

- Loader Multiboot2 (`x86/boot/loader.asm`): carica il kernel, prepara il framebuffer e il long mode.
- Kernel x86-64 (`x86/kernel`, `no_std`) con il desktop (`gui/`).
- App pronte al primo avvio, senza installazione: FILE, TERM (terminale), CALC (calcolatrice), WEB (browser), EDIT (editor), SET (impostazioni), ORA (orologio), READ (lettore), IDE.
- Suite documenti: DOCUMENTI (DOC) mostra il testo di un `.docx` incluso (`office/tests/fixtures/sample.docx`, letto con `haiku-office`). Sola lettura, un solo documento.
- Giochi: SCACCHI (gioco a due, cursore con le frecce, Invio per muovere). Regole da `gba/src/chess.rs`: niente arrocco né en passant, promozione automatica a donna.
- Non incluso: fogli, presentazioni, PDF e apertura di file dal disco.

## Come si crea

Requisiti (Debian/Ubuntu):

```sh
sudo apt-get install -y qemu-system-x86 nasm binutils grub-pc-bin grub-common xorriso mtools
rustup target add x86_64-unknown-none
```

Comando (da `x86/`):

```sh
./scripts/mkiso.sh        # produce build/haikuos.iso
```

## Come si avvia

- **QEMU:** `qemu-system-x86_64 -cdrom build/haikuos.iso -m 256M -vga std`
- **VirtualBox:** nuova VM, tipo "Other / Unknown (64-bit)", 256 MB RAM, grafica VBoxVGA o VMSVGA, collega `haikuos.iso` come CD e avvia.
- **Controllo automatico:** `./scripts/test-iso.sh` (QEMU headless, seriale in `build/iso_serial.log`, screenshot in `build/iso_screen.ppm`).

## Note

- Nella ISO non c'è un disco ATA con il file system di prova `HAIKUFS1`: sul seriale compaiono due righe `[errore]`. Il desktop non le usa.
- `grub.cfg` chiede il modo grafico 1280x800x32. Se la VM non lo supporta, il loader non trova il framebuffer e il desktop non parte.
