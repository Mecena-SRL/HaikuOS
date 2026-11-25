# App di sistema e produttività

> Piano e prototipo. Ogni misura non fatta è segnata **da verificare**.
> Base: [base-programs.md](base-programs.md) (regola: un processo, una superficie, quote dal monolite), [desktop-concept.md](desktop-concept.md) (tinte, tasti), [graphics.md](graphics.md) (superfici).

## Regole comuni

- **Logica pura prima**: ogni app è un modulo senza disegno, testato sull'host (`gba/host/test_apps.rs`). Il disegno va sulla superficie dopo.
- **Memoria**: solo quota concessa dal monolite. Niente allocazioni dinamiche (`no_std`, `core` soltanto).
- **Dati**: passano da `Fs` (lettura) e dalla quota di scrittura. Nessun accesso diretto all'hardware.
- **Un tasto per azione**: croce per scegliere, A conferma, B indietro, Start esce.

## Elenco

Risorse: RAM = stima di memoria statica; FS = file system; Cifr. = crittografia (Ascon).

| # | App | Scopo | Risorse | Dipendenze | Priorità | Verificabile sul GBA |
|---|---|---|---|---|---|---|
| 1 | Editor di testo | Modifica un file di testo | RAM ~140 B, FS r/w | `editor.rs`, `fs.rs` | Quasi pronto | Dimensione massima file (8 KB) |
| 2 | Note | Più note di una riga, fino a 8 | RAM ~1,2 KB (8 editor), FS r/w | `editor.rs` | **Collegata** (`apps/notes.rs`) | Quanti caratteri entrano su schermo |
| 3 | Orologio | Ora su 24 ore, HH:MM | RAM 8 B, timer/frame | nessuna | **Collegata** (`apps/clock.rs`) | Deriva dei 60 fps contro i ~59,73 reali: **da verificare** |
| 4 | Impostazioni di sistema | Tema (4 palette), nome del sistema | RAM 16 B, FS (salvataggio) | `desktop-concept.md` | **Collegata** (`apps/settings.rs`) | Palette su schermo reale: da verificare |
| 5 | Archiviatore | Crea, legge, decomprime archivi; cifra file e cartelle | Contenitore fatto; RAM e CPU da misurare | FS, Cifr. (Ascon), quote | Dopo 2–4 | Tempo di cifratura: da verificare |
| 6 | Gestore password | Archivio cifrato di credenziali | RAM ~ decine di KB (da verificare) | Cifr., FS | Dopo 5 | Dimensione archivio in SRAM |
| 7 | Terminale | Shell di base (`ls`, `cd`, `cat`) | Superficie a caratteri | FS, superficie | In corso (altro thread) | Griglia 30×16: da verificare |
| 8 | File manager | Elenca e apre directory | FS (lettura) | FS | In corso (altro thread) | 16 voci a schermo |
| 9 | Utility disco | Stato dello store, spazio usato | FS (sola lettura) | FS | Media | Costo del conteggio: da verificare |
| 10 | Info di sistema | Versione, memoria, slot, avvii | Monolite (sola lettura) | monolite | Media | Dati già disponibili |
| 11 | Promemoria e calendario | Voci con data e ora | RAM, FS, orologio | orologio (3) | Dopo 3 | Fuso orario: da decidere |
| 12 | Dizionario | Parole e definizioni in sola lettura | FS (dati grandi) | FS | Bassa | Spazio: un dizionario non entra in 32 KB (da verificare) |
| 13 | Schermata di blocco | Ora e blocco con PIN | RAM minima, orologio | orologio (3) | Aggiunta | Input del PIN con croce |
| 14 | Cestino | File cancellati, recuperabili | FS (marcatura, non copia) | FS | Aggiunta | Costo sulla SRAM |
| 15 | Ricerca nel file system | Cerca per nome | FS (lista) | FS | Aggiunta | Tempo di scansione: da verificare |
| 16 | Centro notifiche | Coda di messaggi di sistema | RAM (coda fissa) | monolite | Aggiunta | Dimensione coda |
| 17 | Script editor | Scrive ed esegue script brevi | Terminale, FS | terminale (7) | Dopo 7 | Interprete: da decidere |
| 18 | Screenshot | Salva la superficie attiva | FS, superficie | compositore | Bassa | Dimensione (240×160 ×2 B ≈ 75 KB, da verificare) |

Fuori da questo documento (altre categorie): browser, telefono, scacchi, player, foto, ebook, musica, messaggi, rubrica, mail, mappe, memo vocali, colorimetro, MIDI, stampante, condivisione schermo.

## Ordine di lavoro

1. Editor di testo e Note (riusano `editor.rs`).
2. Orologio e Impostazioni.
3. Archiviatore (serve la cifratura Ascon: prima va misurata).
4. Il resto, con priorità da rivedere dopo le misure su hardware.

## Prototipo

- **Collegate al desktop (ROM)**: Orologio (CLOCK, anche nella barra) e Impostazioni (SETTINGS). Tasti:
  - Orologio: su/giù ±1 minuto, destra/sinistra ±1 ora, B chiude.
  - Impostazioni: destra/sinistra cambia tema (4 palette, le altre tre sono **provvisorie**), B chiude. Il nome si legge ma non si modifica ancora.
- **Note non collegate**: l'icona NOTES apre l'editor a schermo intero già su main (altro thread). `apps/notes.rs` (più note) resta pronto ma non è nel desktop: da decidere se sostituisce quell'editor.
- `gba/src/apps/`: logica (`notes.rs`, `clock.rs`, `settings.rs`), disegno nel corpo finestra (`view.rs`), contenitore archivio (`archive.rs`, senza compressione né cifratura).
- Test host: `cd gba && rustc --edition 2021 --test host/test_root.rs -o /tmp/gba_t && /tmp/gba_t` (71 test, tutti ok) e `host/test_apps.rs` (13 test).
- Screenshot: test ignorato `screenshot_app_di_sistema` (`OUT=cartella /tmp/gba_t screenshot_app_di_sistema --ignored`), stessa `paint` della ROM, senza emulatore.
- Archiviatore: solo il contenitore (`apps/archive.rs`, 2 test). Compressione e cifratura da fare.

### Misura Ascon (host x86, `sim/tests/ascon_misura.rs`)

Dati ripetitivi (LZSS li comprime bene): i tempi sono ottimistici.

| Dimensione | Seal (LZSS + Ascon) | Open | Cifrato |
|---|---|---|---|
| 1 KB | 126,5 µs | 8,7 µs | 436 B |
| 4 KB | 272 µs | 20,5 µs | 896 B |
| 8 KB | 467 µs | 20,7 µs | 1511 B |

Su ARM7TDMI il costo sarà molto più alto: **da verificare**.

### Misure in mGBA (libmgba 0.10.2): non ancora confermate

- Ascon-AEAD128 per blocchi da 64 byte è in ROM (`gba/src/ascon.rs`, test host su vettori di `sim/src/slot.rs`). Nel boot la schermata delle misure mostra seal e open, ma **non abbiamo ancora letto con sicurezza il valore dallo schermo emulato**: le prime letture (circa 161 000 cicli per seal, 147 000 per open per blocco) non sono verificate.
- Il controllo di apertura sulla ROM non è confermato: il rettangolo di esito non compare nei fotogrammi campionati.
- Prossimo passo: leggere i valori dalla memoria dell'emulatore invece che dai pixel.

## Da verificare

- Consumo reale di RAM e stack su ARM7TDMI.
- Frequenza dei frame del GBA per l'orologio (59,73 Hz nominali, da misurare).
- Palette alternative del tema.
- Tempi della SRAM per salvare note e impostazioni.
