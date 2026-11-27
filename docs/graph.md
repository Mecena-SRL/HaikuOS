# Grafo del progetto (Graphify)

Mappa viva di moduli e dipendenze. Va aggiornata ad ogni PR.

## Moduli

| Modulo | Tipo | Dipende da | Ruolo |
|---|---|---|---|
| `kernel/` | OS, x86 32 bit | - | kernel Multiboot2, primo avvio su COM1 |
| `setup/` | OS, `no_std` | - | logica primo avvio |
| `sysapps/` | OS, `no_std` | - | browser ed editor di script |
| `sim/` | host | - | simulatore monolite, agente, filtro messaggi |
| `gba/` | OS, Game Boy Advance | - | ROM GBA |
| `x86/` | OS, x86-64 | - | loader Multiboot2 (CPUID, framebuffer 15/16/24/32 bit), kernel `no_std`, test QEMU |
| `arm64/uefi/` | OS, ARM64 UEFI | - | app UEFI (`BOOTAA64.EFI`), GOP, stampa su ConOut, test QEMU_EFI + ramfb |
| `arm64/` | OS, ARM64 | - | kernel `no_std` su QEMU virt: EL1, seriale PL011, timer generico (polling), MMU identità (2 GB, blocchi da 1 GB) |
| `gui/` | host | - | desktop e window manager |
| `office/src/ods.rs` | host, no_std | formats | ODS in sola lettura, test inline |
| `sim/` (formati) | - | - | rimossi: duplicavano `office/` |
| `office/src/snappy.rs` | host, no_std | - | Snappy a blocchi, test inline |
| `office/src/pages.rs` | host, no_std | snappy, zip | testo Pages da IWA, **da verificare** |
| `devtools/` | host, std | - | manifest, ordine di build, build runner, package manager, nucleo IDE, progetto, SHA-256, CLI `hdev` |
| `tools/recompiler/` | host, std | serde_json | ricompilatore C -> Rust via LLM, verifica con `cargo test`; vedi `docs/recompiler.md` |

## Stato

- `x86/`: CPU (long mode, SSE2) e framebuffer generico (#204 unita). 16 bit non verificato in QEMU.
- `arm64/`: avvio EL1, seriale PL011 (#205 unita). Timer generico (polling), MMU identità (#206 unita). GICv2 e interrupt UART per la tastiera (#207 unita). `arm64/uefi/`: app UEFI verificata in QEMU: questa PR. Timer con interrupt (PPI 30) (#209 unita). GOP (#210 unita). `arm64/uefi/src/gfx.rs`: formato pixel generico (RGB8, BGR8, maschere 16/24/32 bit), desktop con etichette (font da `gba/src/font.rs`, #212 unita). `input.rs`: tastiera UEFI, launcher (#213 unita). `calc.rs`: calcolatrice (#214 unita). `term.rs`: terminale a riga (#215 unita). `rtc.rs`: orologio PL031 (#216 unita). Temi e app SET (#217 unita). `docs.rs` e `editor.rs`: testi incorporati, editor; FILE, READ, DOC, EDIT, WEB: questa PR. `fs.rs`: file system UEFI (FAT sulla ESP), FILE/READ/EDIT con salvataggio: questa PR. `net.rs`: scheda di rete SNP del firmware (MAC, collegamento), WEB mostra lo stato: questa PR. `tcp.rs`: stack minimo ARP, IPv4, TCP client, HTTP/1.0 GET; WEB con `G`: questa PR. Non ancora: DNS, HTTPS, hardware reale.
- `devtools/`: manifest, ordine di build, verifica SHA-256 (#158 unita).
- Build runner: `build.rs` + `hdev` (#162 unita).
- Package manager: `pm.rs` (indice, risoluzione, installazione atomica; questa PR).
- IDE: nucleo `ide.rs` (buffer con annulla) e `project.rs` (file e load/save; questa PR). GUI da fare: la finestra la fornisce il desktop.

## Suite office (`office/`, `no_std`, solo `core`)

| Modulo | Dipende da | Ruolo |
|---|---|---|
| `text.rs` | - | testo a dimensione fissa, formato `.txt` |
| `inflate.rs` | - | DEFLATE senza allocazioni |
| `zip.rs` | `inflate` | ZIP in lettura (stored, DEFLATE) e scrittura (stored) |
| `formats.rs` | `zip` | testo di docx e odt, scrittura docx e odt |
| `pdf.rs` | `inflate` | PDF in sola lettura (font semplici e Identity-H) |
| `sheet.rs` | `zip`, `formats` | fogli: CSV, xlsx, formule |
| `slides.rs` | `zip`, `formats` | presentazioni: formato nativo, lettura e scrittura pptx |

Stato: tappe 1-3 unite (#121, #171, #174). Scrittura pptx minima verificata con LibreOffice. Rinviato: `.pages`.
