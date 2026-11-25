# App media e lettura

> Piano e prototipo host. Ogni misura non provata è segnata **da verificare**.
> Base: [base-programs.md](base-programs.md) (regola: un processo, una superficie), [resource-microkernels.md](resource-microkernels.md) (audio, GPU, storage).

## Regola comune

- Nessun accesso diretto all'hardware: audio, grafica e file passano dai microkernel con quota.
- Ogni app regge il rifiuto di una quota (`Denied`) senza bloccarsi.
- Prima la logica pura (testabile sull'host), poi la superficie.

## Elenco

| # | App | Scopo | Microkernel | Formati | Sul GBA | Stato |
|---|---|---|---|---|---|---|
| 1 | Scacchi | Partita locale, mosse legali, matto e stallo | nessuno (logica pura), superficie | n/a | Sì: 8×8 su una superficie, mosse con keypad (**da verificare** lo spazio in RAM) | Prototipo: `sim/src/chess.rs` |
| 2 | Ebook (testo) | Lettura paginata di testo semplice | storage (lettura), superficie | TXT, UTF-8 | Sì per TXT corto; file grandi no (**da verificare**: 8 KB nello store) | Prototipo: `sim/src/ebook.rs` |
| 3 | Lettore audio | Riproduzione di PCM grezzo | audio (mixer, canale), storage | PCM i16 mono (WAV senza compressione) | Forse: solo campioni brevi in SRAM/ROM (**da verificare**) | Prototipo: `sim/src/player.rs` sul mixer |
| 4 | Libreria musicale | Elenco e riproduzione di brani | audio, storage | come 3 | Solo lista | Da fare, dopo 3 |
| 5 | Visualizzatore immagini | Mostra immagini | GPU/video, storage | 15 bit grezzo; PNG/JPG da fare | Sì per 15 bit piccoli (vedi `docs/base-programs.md`) | Da fare |
| 6 | Lettore foto e documenti | Anteprime di foto e documenti | GPU/video, storage | come 5, più PDF | Solo anteprime piccole | Da fare |
| 7 | Libreria fotografica | Indice e miniature delle foto | storage, GPU/video | come 5 | No (miniature e indice troppo grandi) | Da fare |
| 8 | Lettore video | Riproduzione video | GPU/video, audio, storage | da definire | **No** | Da fare, solo ARM64/x86 |
| 9 | Memo vocali | Registrazione e riproduzione | audio (ingresso e uscita), storage | PCM | **No** (microfono assente sul GBA: **da verificare**) | Da fare |

## Su GBA

- Scacchi e lettore sono finestre del lanciatore (icone CHESS e READ), sulla superficie della calcolatrice (`gba/src/media_ui.rs`, logica in `gba/src/chess.rs`).
- Screenshot in `docs/screenshots/` (06-09), letti dalla VRAM con libmgba 0.10.2.

## Prototipi (sull'host)

1. **Scacchi** (`sim/src/chess.rs`): mosse pseudo-legali e legali, scacco, matto, stallo, promozione automatica a donna. Arrocco ed en passant **da fare**. Test perft: 400, 8902, 197 281 alla profondità 2, 3, 4 (`sim/tests/media.rs`).
2. **Ebook** (`sim/src/ebook.rs`): a capo per parola, parole lunghe spezzate, pagine, avanti e indietro. Conta i caratteri, così gli accenti pesano uno.
3. **Lettore audio** (`sim/src/player.rs`): carica PCM nel canale del mixer a blocchi, in pausa non accoda. Mixer con `free()` aggiunto in `sim/src/audio.rs`.

Test: `cd sim && cargo test --test media` (15 test, tutti passati; circa 35 s in debug per il perft a profondità 4).

## Scelte

- **Scacchi:** niente arrocco ed en passant nel prototipo per tenerlo corto. Promozione sempre a donna. Su hardware la logica va portata a `no_std` + array fisso (oggi usa `Vec`).
- **Ebook:** solo testo semplice. Niente EPUB (zip e XML sono troppo grandi per il GBA). EPUB su ARM64/x86 **da verificare**.
- **Audio:** niente decodifica (mp3, ogg, flac). Il decodificatore è un passo successivo, da scegliere fuori dal GBA. Canali del mixer: massimo 4, come da `resource-microkernels.md`.

## Da verificare

- Quanta RAM serve per una partita di scacchi con storico mosse sul GBA.
- Dimensione massima di un file ebook nello store (oggi 8 KB totali).
- Latenza del mixer: 64 campioni per canale bastano per non avere scatti? (`docs/resource-microkernels.md`).
- Registrazione vocale sul GBA: microfono assente, quindi memo vocali solo su ARM64/x86.

## Prossimi passi

1. Superficie per scacchi e ebook (disegno su buffer, senza logica nuova).
2. Decodifica WAV nel lettore audio.
3. Libreria musicale come elenco di file WAV.
