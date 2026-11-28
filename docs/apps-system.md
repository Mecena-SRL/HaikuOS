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
| 6 | Gestore password | Archivio cifrato di credenziali | RAM ~ decine di KB (da verificare) | Cifr., FS | Sblocco, voci (nuova, modifica) e salvataggio nel file `vault` (`apps/vault.rs`, Ascon). Sale da seme di sistema (entropia debole, da migliorare) | Costo KDF (`ITER`=512) su ARM7TDMI; dimensione in SRAM |
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

### App collegate (lanciatore e terminale)

Il lanciatore ha 19 icone su una pagina (5 colonne, 4 righe). Il terminale apre ogni app digitandone il nome (SELECT, lettere, sinistra). Stato e tasti in `apps/ui.rs`, disegno in `apps/view.rs`.

| App | Icona (indice) | Comando | Schermata e tasti | Da verificare su GBA |
|---|---|---|---|---|
| Gestore password | PASSWORDS (9) | PASSWORDS | Una voce alla volta, su/giù, A mostra il segreto (dato di prova) | Lettura dal campo cifrato: da fare |
| Dizionario | DICTIONARY (10) | DICTIONARY | Parola e definizione a capo, su/giù | Dizionario completo: da verificare |
| Calendario | CALENDAR (11) | CALENDAR | Griglia del mese, sinistra/destra cambia mese, un evento di prova | Fuso orario (UTC): da decidere |
| Messaggi | MESSAGES (12) | MESSAGES | **DA VERIFICARE** (solo hardware) | Collegamento seriale: da verificare |
| Foto | PHOTOS (13) | PHOTOS | Elenco a pagine, su/giù, sinistra/destra cambia pagina | Miniature: da fare |
| Visualizzatore foto | VIEWER (14) | VIEWER | Riquadro della foto a zoom, croce sposta, A zoom avanti, Select indietro | Immagine reale: da fare |
| Fotocamera | CAMERA (15) | CAMERA | **DA VERIFICARE** (solo hardware) | Il GBA non ha fotocamera: da verificare |
| Musica | MUSIC (16) | MUSIC | **DA VERIFICARE** (solo hardware) | Audio: da verificare |
| Mappe | MAPS (17) | MAPS | Griglia 32×32 con cursore, croce muove (percorso BFS non disegnato: stack da misurare) | Stack del BFS: da verificare |
| Memo | MEMO (18) | MEMO | **DA VERIFICARE** (solo hardware) | Microfono: da verificare |

Dati di prova: una password, un evento, tre foto e una mappa di seme 7. Niente salvataggio su SRAM.

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

## Utility di sistema (`gba/src/apps/sysutil.rs`)

Logica pura, testata sull'host (`host/test_apps.rs`, 7 test nuovi). Ogni utility ha una voce nel lanciatore (icona UTIL, finestra con disco, memoria, coda di stampa e nota di riferimento) e un comando nel terminale. Righe di output massimo 14 caratteri.

| Utility | Terminale | Stato |
|---|---|---|
| Utility disco | `DISCO` | Completa (logica + finestra) |
| Info di sistema | `INFO` | Completa (memoria da costanti del monolite) |
| Stampante | `STAMPA`, `STAMPA FINE` | Coda simulata: il driver della stampante non c'è. **Da verificare** |
| Configuratore MIDI | `NOTA n` | Solo nota → frequenza (decimi di Hz). Nessuna porta MIDI. **Da verificare** |
| Colorimetro | `COLORE rrggbb` | Nome del colore più vicino. Nessun sensore: **da verificare** |
| Screenshot | nessuno | Solo la miniatura 1 bit (`thumb`, 4800 B, entra nei 8 KB dello store). Cattura dal framebuffer **non collegata** |
| Archiviatore cifrato | nessuno | `archive::seal`/`open` con Ascon-AEAD128 e test. Nessuna voce in UI né terminale. **Da verificare**: tempi su ARM7TDMI |

Nota: una foto piena (240×160×2 B ≈ 75 KB) non entra nello store da 8 KB né in uno slot da 32 KB: per questo la miniatura è 1 bit.

## Logica delle utility di sistema (non collegate)

| Modulo | App | Dati in ingresso | Stato |
|---|---|---|---|
| `apps/registri.rs` | Registri | Voci di log (anello fisso, 8 voci) | Logica testata, non nel desktop |
| `apps/connessioni.rs` | Connessioni | Stato rete e IP (`Offline` finché il net stack non c'è) | Logica testata, non nel desktop |
| `apps/dischi.rs` | Dischi | Byte usati e totali per spazio (sistema, utente) | Logica testata, non nel desktop |
| `apps/analisi.rs` | Analizzatore disco | Classifica dei file più grandi (top 3), barra 10 caselle | Logica testata, non nel desktop |
| `apps/caratteri.rs` | Caratteri | Conteggi da `fontmgr` (installati, attivi) | Logica testata, non nel desktop |

Test host: `cd gba && rustc --edition 2021 --test host/test_sistema.rs -o /tmp/sis_t && /tmp/sis_t` (12 test, tutti ok).
Il disegno e il collegamento al lanciatore (`desktop.rs`, `ui.rs`) sono il passo successivo: **da verificare** su schermo e memoria.
