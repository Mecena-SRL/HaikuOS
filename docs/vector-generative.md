# Grafica vettoriale generativa

> Stato: bozza con prototipo. I numeri sono misurati sull'host di sviluppo (x86, build release). Il target low-end è **da verificare**.

## 1. Idea

Gli elementi grafici non sono bitmap né liste fisse. Sono **funzioni di parametri** (dimensione, tema, stato) che producono primitive vettoriali quando servono. Si salvano solo i parametri, pochi byte. Il pixel si calcola al momento, e si riusa il risultato finché i parametri non cambiano.

Obiettivi, in ordine: RAM minima, CPU minima, nessuna allocazione dinamica.

## 2. Primitive

| Primitiva | Stato | Note |
|---|---|---|
| Rettangolo pieno | fatto | `Canvas::rect` (`editor/src/gui.rs`) |
| Contorno 1 px | fatto | `Canvas::frame` |
| Cerchio pieno | fatto | test di distanza intera, nessun float (`editor/src/chrome.rs`) |
| Path/curve generiche | da fare | scan-line con bordi interi, al massimo N segmenti fissi |
| Gradiente | da fare | calcolato per riga, non salvato |
| Contorno glifo | da fare | outline del font generato su richiesta, cache per (glifo, dimensione) |

## 3. Generazione dai parametri

Esempio prototipo: `Chrome { w, bar, focused, bg }` (24 byte).

- `w`: larghezza finestra; `bar`: altezza barra.
- Raggio dei pulsanti = `bar / 4`. Posizioni dei pulsanti = multipli di `bar`.
- `focused` sceglie il colore dei pulsanti (colorati o grigi).

Il tema è un colore nei parametri, non un file. Un cambio di tema cambia i parametri, non gli asset.

## 4. Cache e ridisegno a regioni (dirty rect)

- Ogni elemento tiene i propri parametri precedenti. Se sono uguali, non disegna (costo misurato: ~1,3 ns).
- Se cambiano, si ridisegna solo il rettangolo che li contiene (dirty rect), non tutto lo schermo.
- Cache opzionale: un buffer per elemento, usato solo se l'elemento viene ridisegnato spesso. Costa RAM (vedi tabella), quindi è opzione, non default.
- Niente allocazione nel ciclo: i buffer sono di dimensione fissa, decisa all'avvio.

## 5. Budget per frame

Obiettivo: 60 fps, frame da 16,6 ms. Budget grafico ≤ 1 % del frame per il chrome, cioè ≤ 166 µs su target low-end.

| Scenario | Generato ogni frame | Bitmap salvato (stored) | Cache (invariato) |
|---|---|---|---|
| Chrome 1920×1080, barra 32 px | 8,33 µs CPU, 24 B | 7,21 µs, 245 760 B | 0,0013 µs, 24 B |
| Chrome 3840×2160, barra 32 px | 15,74 µs CPU, 24 B | 15,98 µs, 491 520 B | 0,0014 µs, 24 B |

Lettura: il bitmap salvato non è più veloce del generato (entrambi sono limitati dalla scrittura del buffer). Il vantaggio reale è la RAM: oltre 10 000 volte meno, e con la cache il costo a regime è trascurabile.

Da verificare su hardware low-end (x86 a bassa potenza, e poi ARM64/GBA): stessi numeri, con il budget sopra.

## 6. Mappatura sul Canvas esistente

- Il Canvas attuale è `editor/src/gui.rs` (`Canvas<P>` con `rect`, `frame`, `clear`). Nel repo non esiste `gui/src/render.rs`.
- Il Canvas resta il punto unico di scrittura pixel. Le primitive vettoriali si aggiungono come funzioni sopra `Canvas::px`, senza cambiare il formato `Pixel` (u32 host, u16 GBA).
- `chrome.rs` usa solo `rect` e `px`, quindi funziona anche su GBA (`no_std`, nessun alloc).

## 7. Prototipo misurato

File: `editor/src/chrome.rs`. Test: `cargo test -p haiku-editor`. Benchmark: `cargo test -p haiku-editor --release bench_generated -- --ignored --nocapture`.

Nel repo non esisteva ancora un chrome di finestra. Come baseline ho usato la versione "stored" (bitmap pre-renderizzato della sola barra), cioè quello che si farebbe senza generazione.

## 8. Prossimi passi

1. Path generici con scan-line e gradienti (primitive §2).
2. Contorno glifi generato, con cache per (glifo, dimensione).
3. Misura su hardware low-end reale.
4. Dirty rect nel ciclo `platform::run`.
