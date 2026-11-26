# ISO avviabile di HaikuOS (x86-64, VM)

Stato: avvio verificato in QEMU 8.2 (BIOS, seriale e screenshot del desktop 1280x800). VirtualBox, VMware e hardware reale: **da verificare**.

## Cosa contiene

- Loader Multiboot2 (`x86/boot/loader.asm`): carica il kernel, prepara il framebuffer e il long mode.
- Kernel x86-64 (`x86/kernel`, `no_std`) con il desktop (`gui/`).
- App pronte al primo avvio, senza installazione: FILE, TERM (terminale), CALC (calcolatrice), WEB (browser), EDIT (editor), SET (impostazioni), ORA (orologio), READ (lettore), IDE.
- Suite documenti: DOCUMENTI (DOC), sola lettura. Con ←/→ scegli tra quattro documenti inclusi (`office/tests/fixtures/`): `sample.docx`, `sample.pdf` (testo), `sample.pptx` (diapositive separate da `---`), `sample.xlsx` (come CSV). Scorrimento con ↑/↓.
- Giochi: SCACCHI (gioco a due, cursore con le frecce, Invio per muovere). Regole da `gba/src/chess.rs`: niente arrocco né en passant, promozione automatica a donna.
- Non incluso: modifica dei documenti, scrittura nei formati office, apertura di file scelti dall'utente (nella ISO non c'è un disco).
- Kernel: circa 100 KB. Il disco di prova (`build.sh`) carica il kernel a blocchi da 32 KB, quindi il limite è 1 MB.

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

## VirtualBox

`x86/scripts/vbox-create.sh` crea la VM "HaikuOS" (BIOS, 256 MB, VGA 1280x800) con `build/haikuos.iso` nel lettore DVD. Poi: `VBoxManage startvm HaikuOS`.

Stato: la VM viene creata e registrata (verificato). L'avvio non è verificato: nell'ambiente di sviluppo VirtualBox è installato ma senza modulo kernel (`vboxdrv`) e senza KVM, quindi non può far partire VM.
