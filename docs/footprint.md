# Ingombro del sistema (GBA)

> Misura del 2026-11-25, su `main` (dopo PR #115). Compilazione `--release`, toolchain `rustc 1.102.0-nightly (739adecd4 2026-11)`.
> Sono byte del binario, non dell'emulatore. Da rifare a ogni modifica sostanziale.

## Comando

Dalla cartella `gba/`:

```sh
cargo build --release --target thumbv4t-none-eabi
ELF=target/thumbv4t-none-eabi/release/haiku-gba
/usr/bin/llvm-objcopy -O binary $ELF haiku-gba.gba     # ROM
/usr/bin/llvm-size -A $ELF                              # sezioni (.text = ROM, .bss = RAM statica)
/usr/bin/llvm-nm -S -C --size-sort $ELF > /tmp/nm.txt   # dimensione dei simboli, per modulo
```

Il raggruppamento per modulo somma i simboli `t` il cui nome contiene `haiku_gba::<modulo>`. Le funzioni inline finiscono nel chiamante: `rust_main` contiene il boot, il workspace e il desktop.

## ROM

| Voce | Byte |
|---|---|
| `haiku-gba.gba` (`.text`, nessun dato iniziale) | 25 176 |
| Limite della cartuccia (ROM) | 32 MiB |

## RAM statica

| Sezione | Indirizzo | Byte | Contenuto principale |
|---|---|---|---|
| `.bss` | EWRAM 0x02000000 | 46 176 | `SURFACES` 24 640, `CALC_SURF` 15 392, `MEDIA_SURF` 6 144 |
| `.data` | IWRAM | 0 | nessuno |
| IWRAM libera per lo stack | 0x03000000–0x03007F00 | ≈ 32 000 | **non misurato**: lo stack non è stato misurato |

EWRAM totale: 256 KiB. Usata staticamente: 46 176 byte (≈ 18 %).

## Codice per modulo (byte di `.text`)

| Modulo (sorgente) | Byte |
|---|---|
| `rust_main` (`main.rs`, incluso l'inline) | 7 316 |
| `persist` (`persist.rs`) | 3 906 |
| `desktop` (`desktop.rs`) | 3 088 |
| `compiler_builtins` / `core` | 2 670 |
| funzioni di `main.rs` (editor, disegno) | 1 520 |
| `fs` (`fs.rs`) | 1 508 |
| `chess` (`chess.rs`) | 1 126 |
| `font` (`font.rs`) | 900 |
| `terminal` (`terminal.rs`) | 728 |
| `monolith` (`monolith.rs`) | 494 |
| `glyph` (`glyph.rs`) | 212 |
| `calc` (`calc.rs`) | 198 |
| `space` (`space.rs`) | 152 |
| `media_ui` (`media_ui.rs`) | 140 |
| `apps` (`apps/`) | 112 |
| `surfaces` (`surfaces.rs`) | 76 |
| **Totale simboli** | **24 146** |

Il totale dei simboli (24 146) è meno della ROM (25 176): la differenza sono i simboli non attribuiti (tabelle, allineamenti, intestazione).

## Da verificare

- Dimensione dello stack in uso, con misura in mGBA (riempire IWRAM di un pattern e leggerlo a fine esecuzione).
- Dipendenza dei moduli inline: `rust_main` accorpa il boot e i loop, quindi il dato per modulo sopra sottostima il costo dei singoli programmi.
