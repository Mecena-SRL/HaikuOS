# Grafica vettoriale generativa

> Stato: bozza. Misure sull'host di sviluppo (x86, release), non sul target low-end: **da verificare**. Regole grafiche: [graphic-bible.md](graphic-bible.md). Codice: `gui/`.

## 1. Idea

Gli elementi grafici sono **funzioni di parametri** (misure, tema, stato) che disegnano sul `Canvas` quando servono. Non si salvano bitmap né liste fisse di elementi. Per i casi ripetuti si può tenere un risultato in cache, invalidato quando i parametri cambiano.

Ordine delle priorità: RAM minima, poi CPU, nessuna allocazione nel ciclo.

## 2. Primitive (già nel trait `Canvas`, `gui/src/render.rs`)

| Primitiva | Metodo | Stato |
|---|---|---|
| Rettangolo pieno | `fill` | fatto |
| Rettangolo arrotondato | `rect_rounded` (x86; GBA retto) | fatto |
| Linea spessa | `line` | fatto |
| Testo | `text`, `vtext` (x86 vettoriale) | fatto |
| Icona vettoriale | `icon_vec` (rettangoli su griglia 16×16) | fatto |
| Sfumatura/trasparenza | `blend` | fatto |
| Path generico, gradiente | — | da fare |

## 3. Generazione dai parametri

- Le misure dei pulsanti vengono da funzioni che partono dal frame: `close_button`, `max_button`, `min_button`, `title_bar` (`gui/src/layout.rs`). Cambiando il frame, cambia tutto il chrome, senza asset.
- Il tema è un token (`Tok::T0`–`T3`, `Tok::A(n)` per accenti) nella palette, non un file.
- Il disegno della scrivania è `render::draw` / `draw_motion`: legge lo stato `Shell` e disegna tutto.

## 4. Cache e ridisegno a regioni

- `gfx::Dirty<N>` (già presente) raccoglie i rettangoli sporchi; `gfx::copy` copia solo quelli.
- Se lo stato non è cambiato, il frame precedente resta valido: nessun disegno.
- Il frame intero salvato costa RAM (vedi §5). Va usato solo dove il disegno è davvero lento.

## 5. Misure (prototipo: `gui/tests/chrome_bench.rs`)

Scena: una finestra aperta, chrome completo (titolo, min/max/chiudi). Canvas di prova `Buf` (un `Tok` per pixel), non il framebuffer reale.

| Risoluzione logica | Generato per frame | Frame salvato (copia) | RAM salvato | RAM generatore |
|---|---|---|---|---|
| 240×160 (GBA) | 44,39 µs | 1,73 µs | 76 800 B | 0 B extra |
| 640×480 (x86) | 278,34 µs | 13,21 µs | 614 400 B | 0 B extra |

Lettura: generare è circa 20 volte più lento della copia, ma non occupa RAM. Il salvato è più veloce in CPU e costa un buffer intero. Il dato che conta è il frame reale: il `Canvas` del framebuffer x86 (`x86/kernel/src/fb.rs`) va misurato a parte.

Budget (da verificare su hardware): a 60 fps un frame dura 16,6 ms. Per la sola barra e i pulsanti, ≤ 1 % (166 µs) su low-end.

## 6. Prossimi passi

1. Misurare il `Canvas` reale su x86 (QEMU) e su GBA.
2. Dirty rect nel ciclo di disegno della scrivania, invece del frame intero.
3. Path generici e gradienti come primitive (§2).
