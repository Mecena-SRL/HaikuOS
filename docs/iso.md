# ISO avviabile di HaikuOS (x86-64, VM)

Stato: avvio verificato in QEMU 8.2 con BIOS e con firmware UEFI (OVMF), seriale e screenshot del desktop 1280x800. VirtualBox, UTM, VMware e hardware reale: **da verificare**.

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

- **QEMU (BIOS):** `qemu-system-x86_64 -cdrom build/haikuos.iso -m 256M -vga std`
- **QEMU (UEFI):** `qemu-system-x86_64 -machine q35 -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd -drive if=pflash,format=raw,file=OVARS.fd -cdrom build/haikuos.iso -m 512M -vga std` (`OVARS.fd` è una copia di `OVMF_VARS_4M.fd`)
- **VirtualBox:** nuova VM, tipo "Other / Unknown (64-bit)", 256 MB RAM, grafica VBoxVGA o VMSVGA, collega `haikuos.iso` come CD e avvia.
- **Controllo automatico:** `./scripts/test-iso.sh` (QEMU headless, seriale in `build/iso_serial.log`, screenshot in `build/iso_screen.ppm`).

## Note

- Nella ISO non c'è un disco ATA con il file system di prova `HAIKUFS1`: sul seriale compaiono due righe `[errore]`. Il desktop non le usa.
- `grub.cfg` chiede il modo grafico 1280x800x32. Se la VM non lo supporta, il loader non trova il framebuffer e il desktop non parte.

## VirtualBox

`x86/scripts/vbox-create.sh` crea la VM "HaikuOS" (BIOS, 256 MB, VGA 1280x800) con `build/haikuos.iso` nel lettore DVD. Poi: `VBoxManage startvm HaikuOS`.

Stato: la VM viene creata e registrata (verificato). L'avvio non è verificato: nell'ambiente di sviluppo VirtualBox è installato ma senza modulo kernel (`vboxdrv`) e senza KVM, quindi non può far partire VM.

## File dell'utente (disco di prova)

FILE mostra i file del disco HAIKUFS1 (`x86/disk/`): ogni file della cartella entra nel disco, dopo `LEGGIMI.TXT`. Massimo 15 file, nomi fino a 15 caratteri, testo fino a 8 KB in totale.

Per aggiungere file: copiali in `x86/disk/`, poi `./x86/scripts/build.sh` e avvia `build/haikuos.img` (disco master IDE). La ISO non ha disco: mostra i file incorporati.

## Avvio UEFI e framebuffer

- La ISO è ibrida: GRUB EFI (`/EFI/BOOT/BOOTX64.EFI`) e GRUB BIOS. Richiede `grub-efi-amd64-bin` oltre a `grub-pc-bin`.
- Il loader non chiede una risoluzione: GRUB imposta 1280x800x32 (`grub.cfg`) e passa il framebuffer nei tag Multiboot2. Il loader mappa la memoria 0..4 GB, così il framebuffer funziona anche a indirizzi come 0x80000000.
- Limite: un framebuffer sopra 4 GB non è ancora mappato. Se manca il framebuffer, il kernel scrive sul seriale e si ferma.
- Al boot il messaggio "HaikuOS: avvio in corso..." compare solo per un attimo, poi il desktop.
