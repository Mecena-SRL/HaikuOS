# Sistema grafico (GUI comune)

> Regole: [graphic-bible.md](graphic-bible.md). Codice: `gui/`. Ogni valore non misurato su hardware è **da verificare**.

## 1. Cosa c'è (fatto, testato su host)

Crate `gui/` (`haiku-gui`), `no_std`, senza heap, stesso codice per GBA e x86:

| Modulo | Contenuto |
|---|---|
| `palette.rs` | 4 token T0–T3, valori GBA 15 bit e sRGB; test che i due coincidono |
| `layout.rs` | Misure della bibbia: barra 12, titolo 10, chiudi 8×8, icona 16, griglia 4 col. passo 48×40 da (8,24) |
| `wm.rs` | Pila finestre (max 6), apertura centrata, portare in primo piano, chiudi, passa finestra, hit test |
| `shell.rs` | Regole di input (tasti e puntatore), icone, focus desktop/finestra, nessun disegno |
| `render.rs` | Disegno su trait `Canvas` (`fill`, `text`): barra, icone (normale/selezionata/aperta), finestre (titolo attivo T2, inattivo T1) |

Tasti (come `desktop-concept.md` §5):
- Desktop: croce sceglie icona, **A** apre (o porta in primo piano se già aperta).
- Finestra: **B** chiude, **Select**/**R** cambia finestra, **Start** va al desktop, il resto va all'app (`Event::ToApp`).
- Puntatore: click icona = apre; click su chiudi = chiude; click su contenuto = `Event::Tap` con coordinate relative; click su finestra non in cima = la porta in primo piano.

Test (`cargo test` in `gui/`): 
- `bible.rs`: token, misure, etichette maiuscole.
- `usability.rs`: ogni app aperta in ≤ 5 tasti (griglia 9 icone); chiudi = 1 tasto; cambio finestra = 1 tasto; riaprire non duplica.
- `render.rs`: pixel attesi di barra, bordi, titolo attivo/inattivo, chiudi, corpo.

## 2. Cosa manca

1. **GBA**: `gba/src/desktop.rs` (939 righe) usa ancora la sua logica. Va sostituito da `haiku-gui` con un `Canvas` sopra il buffer 240×160 e il font 3×5 di `gba/src/font.rs`. Non fatto: il toolchain ARM non è installato qui, quindi non è verificabile in questa sessione.
2. **x86 framebuffer**: `x86/kernel` ha solo testo VGA 80×25 (`console.rs`). Serve modo grafico (VBE in `stage2.asm`, o GOP/UEFI) e un `Canvas` per il framebuffer. Non esiste ancora.
3. **Scala grande**: `gba/src/scale.rs` ingrandisce il 240×160 con fattore intero. Su x86 il layout va rifatto con la stessa griglia moltiplicata (bibbia §9), non ancora collegato.
4. **Finestre**: niente trascinamento né ridimensionamento (fuori dalla prima versione, come da concept).
5. **Testo**: solo 3×5 maiuscole. Niente vettoriale.
6. **Orologio** nella barra: non presente (serve il timer). Da verificare.
7. **Hardware**: nessuna verifica su Game Boy Advance né su x86 reale o QEMU. Tutto il comportamento grafico è verificato solo da test sui pixel su host.

## 3. Da verificare

- Leggibilità del 3×5 su schermo GBA reale (T2 su T1).
- Tasti: 5 tasti per un'app è accettabile con 9 icone; con più icone il limite cresce (da misurare con utenti reali).
- Dimensione di `Wm` (6 finestre × 20 byte circa) dentro la RAM del GBA: da misurare con il linker.

## 4. Ordine suggerito

1. Adattatore GBA (`Canvas` su VRAM) e sostituzione di `desktop.rs`, con build GBA verificata.
2. Modo grafico x86 (VBE) e `Canvas` su framebuffer, poi QEMU.
3. Test di usabilità con una persona (non solo conteggio tasti).
