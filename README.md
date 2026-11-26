# HaikuOS

Codice del sistema. Documentazione di progetto: [docs/architecture.md](docs/architecture.md).

## Struttura

- `setup/`: logica del primo avvio (nome host, lingua, fuso orario, utente admin, hash password). Senza dipendenze di hardware, testabile sull'host.
- `kernel/`: kernel x86 a 32 bit, avviato da GRUB via Multiboot2. Oggi esegue solo il primo avvio su seriale COM1.
- `tools/`: script di build (`make-iso.sh`) e di prova in QEMU (`test-first-boot.py`).

## Comandi

```sh
cargo test                  # test della logica di setup (host)
sh tools/make-iso.sh        # crea target/haiku.iso
python3 tools/test-first-boot.py   # avvia in QEMU e percorre il primo avvio
sh tools/build-pkgs.sh gba <dir>...   # compila pacchetti (haiku.toml) con hdev, in ordine di dipendenza
```

Requisiti: Rust con target `i686-unknown-linux-gnu`, `grub-mkrescue` (con `xorriso`), `qemu-system-i386`.

## Note

- La password non viene mai salvata in chiaro: solo PBKDF2-HMAC-SHA256 (600.000 iterazioni) con sale da RDRAND.
- La configurazione viene stampata su seriale. Il salvataggio su disco non c'è ancora: manca un driver disco e un filesystem.
- Il kernel è a 32 bit per semplicità di avvio. Il passaggio a x86_64 (modalità lunga) è il prossimo passo.
