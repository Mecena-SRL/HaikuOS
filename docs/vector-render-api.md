# API di rendering vettoriale condiviso

> Stato: nota di design. Il codice è in `gui/src/render.rs` (trait `Canvas`); le aggiunte vettoriali sono **proposte, da verificare**.
> Asset HD di riferimento: `design/hd/` (mockup SVG, generatore `gen.py`, stile condiviso).

## 1. Scopo

Un'unica interfaccia di disegno che i giochi e le app usano senza sapere quale backend c'è sotto (GBA: pixel 240×160; x86/ARM: framebuffer HD). I giochi non chiamano l'hardware: disegnano sulla `Canvas` del proprio programma.

## 2. Trait `Canvas` (già nel codice)

| Metodo | Fa | Note |
|---|---|---|
| `fill(r: Rect, t: Tok)` | riempie un rettangolo con il token colore `t` | base di tutto: sfondi, pulsanti, bordi |
| `text(x, y, s, t)` | scrive testo con il font di sistema | la larghezza del font è responsabilità del backend |
| `rounded() -> bool` | angoli arrotondati delle finestre | solo x86; GBA risponde `false` |
| `rich() -> bool` | accenti per app, pulsanti colorati | solo x86 |
| `icon(cell, label, app)` | icona nella cella (16×16 logici) | default vuoto; il backend decide se c'è l'icona |

Tipi di supporto: `Rect` e `Tok` (token colore della palette). Layout condiviso in `gui/src/layout.rs`: `title_bar`, `body`, `menu_rect`, `icon_cell`, `close_button`, `min_button`, `max_button`.

## 3. Estensioni vettoriali (in parte implementate)

`rect_rounded`, `line`, `icon_vec` e `menu` sono nel trait con implementazione di base (`gui/src/render.rs`, test in `gui/tests/vector.rs`). `icon_vec` riceve rettangoli precompilati da un SVG su griglia 16×16, non parsa SVG a runtime.

| Primitiva | Forma proposta | Uso |
|---|---|---|
| `rect_rounded(r, radius, t)` | rettangolo con raggio | pulsanti, menu, finestre HD |
| `line(x1, y1, x2, y2, w, t)` | segmento di spessore `w` (implementata) | bordi, grafici, orologio |
| `icon_vec(cell, rects, t)` | icona da rettangoli precompilati su griglia 16×16 (implementata) | icone scalabili, nessun PNG a runtime |
| `menu(r, rows, sel)` | menu a righe, riga `sel` evidenziata (implementata) | menu app e contestuali |

Regola: ogni primitiva ha un fallback sul GBA (es. `rect_rounded` diventa `fill`). Un gioco scritto con le primitive base gira ovunque.

## 4. Regole per i giochi

1. Disegnare solo nella propria `Canvas`, mai nel framebuffer.
2. Usare i token della palette, non colori RGB diretti.
3. Chiedere `rounded()` e `rich()` prima di scegliere uno stile.
4. Nessuna allocazione nel ciclo di disegno (budget RAM del GBA).
5. Le icone passano da `icon` o `icon_vec`, mai da file caricati a parte.

## 5. Esempio

```rust
fn draw_score<C: Canvas>(c: &mut C, r: Rect, score: u32, t: Tok) {
    c.fill(r, t);
    c.text(r.x + 4, r.y + 4, "PUNTI", t);
}
```

## 6. Scrivania a piena risoluzione

`gui/examples/desktop_vector.rs` disegna `render::draw` su un `Canvas` a 32 bit, con unità logiche 240×135 e scala intera (16 a 3840×2160, 8 a 1920×1080). Un solo percorso, nessuna dipendenza nuova:

```sh
cd gui && cargo run --release --features wide-font --example desktop_vector -- 3840 2160 out.ppm
```

Output committati: `design/screens/desktop_vector_*.png`. Limiti: i contorni del testo sono ancora il 5×8 vettorizzato, e lo sfondo usa ancora le colline (`hills`), non il wallpaper bitmap di `design/wallpapers/`.

## 7. Font vettoriale con antialiasing

Modulo `gui/src/vfont.rs` (no_std, senza allocazione, senza dipendenze). Metodo del trait: `c.vtext(x, y, size, s, t) -> i32` (ritorna la larghezza in px); `size` = px per em, 1..=64.

Formato dei glifi (byte, coordinate `0..=64` per em, y verso il basso):

| Op | Argomenti | Significato |
|---|---|---|
| `0` | `x y` | move: apre un contorno |
| `1` | `x y` | linea |
| `2` | `cx cy x y` | curva quadratica (4 segmenti) |
| `3` | — | chiude il contorno |
| `4` | `x0 y0 x1 y1` | rettangolo (scorciatoia per 4 linee) |

Riempimento non zero. Copertura: 4 sotto-scanline per riga, area orizzontale esatta, valore 0..255.

Dati: `gui/src/vfont_data.rs` è generato da `gui/tools/gen_vfont.py` dai glifi 5×8 di `x86/kernel/src/font.rs` (ASCII 0x20..0x7E). Passo fisso: 48 unità. Ogni altro byte disegna uno spazio.

Antialiasing sul canvas: `Canvas::blend(x, y, a, t)`. Di base è a soglia (pixel pieno se `a ≥ 128`), quindi GBA e test funzionano. Il backend a 32 bit deve ridefinirlo per mescolare col fondo.

Limiti: al massimo 128 lati per glifo (quelli in eccesso sono scartati), `size ≤ 64`, stack di circa 3 KB per glifo.

Test: `gui/tests/vfont.rs`. Demo: `cargo run --release --example vtext_demo -- 3840 2160 out.ppm`, immagini in `design/screens/vtext_*.png`.

## 8. Animazioni (`gui/src/anim.rs`)

Tutto a interi (ms, permille, µs), no_std, senza allocazione nel frame.

| Tipo | Fa |
|---|---|
| `Ease`, `ease(e, t)` | easing su permille: Linear, InQuad, OutQuad, InOutQuad, OutCubic |
| `Tween` | interpolazione `from`→`to` fra `start` e `start+dur` (ms) |
| `Reactive` | valore con `set(target, now, dur, ease)`: riparte dal valore visualizzato, nessun salto |
| `Slide` | transizione orizzontale tra due schermate: `offsets(now)` dà gli spostamenti |
| `Shifted<C>` | vista di un `Canvas` spostata di `(dx, dy)`: si disegna una schermata come sempre |
| `Budget` | registra µs per frame, conta gli sforamenti di `FRAME_US` (16 667 µs = 60 Hz) |

Scrivania animata (`render::draw_motion` con `Motion { win_dy, sel }`; `draw` resta fermo per GBA):

```sh
cd gui && cargo run --release --features wide-font --example desktop_vector -- 1920 1080 out.ppm 120 open
```

Modi: `open` / `close` (calcolatrice che sale/scende, `Reactive` 250/200 ms), `launch` (selettore che scorre da FILE a CALC, 200 ms). Output: `design/screens/anim_{open,close,launch}_1920x1080.png`, `anim_open_3840x2160.png`. Budget misurato su host (release, x86): 1920×1080 media 3,3 ms, massimo 6,7 ms; 3840×2160 media 10 ms, massimo 25 ms (un frame sopra 16,7 ms). Il testo passa da `vtext`, con copertura reale (`Vector::mix`); i contorni sono ancora quelli 5×8 del font di sistema.
