# Grafica generativa vettoriale

> Stato: prototipo. Codice: `gui/src/genicon.rs`. Esempio: `gui/examples/gen_icon.rs`. Test: `gui/tests/genicon.rs`.
> Non ancora collegato alla scrivania (`render.rs`). Vincoli: no_std, nessuna allocazione, nessun PNG a runtime.

## 1. Idea

Gli elementi grafici non sono disegni salvati ma **formule** (come Vectrex). Il sistema:
1. rileva la superficie (dimensioni, densità, formato pixel);
2. calcola il livello di dettaglio (LOD) dalla dimensione;
3. genera gli elementi dalle formule e li applica alla cache.

## 2. Rilevazione superficie

Dati letti una volta all'avvio o al cambio schermo:

| Campo | Esempio | Uso |
|---|---|---|
| `w`, `h` | 240×160 (GBA), 1920×1080, 3840×2160 | lato minore → LOD |
| `density` | 1× (1080p), 2× (4K) | scala logica → fisica |
| `bpp` | 16/32 bit | colori dei token |

Le unità logiche restano fisse (griglia 16 per icona). Solo il numero di pixel cambia.

## 3. Modello elemento

Un elemento è `f(cella, LOD) → primitivi`:
- primitivo = rettangolo `(x, y, w, h)` + raggio d'angolo + token colore;
- massimo `MAX_PRIMS = 4` per icona (sizing fisso, niente heap);
- coordinate in **griglia 16**: `px = cella.x + k·cella.w/16`, come `icon_vec`.

Esempio, cartella (`genicon::folder`):

| Parte | Griglia (x0,y0 → x1,y1) | Token | LOD minimo |
|---|---|---|---|
| retro | 1,4 → 15,13 | T1 | L0 |
| linguetta | 1,2 → 7,5 | T1 | L0 |
| falda | 1,6 → 15,14 | T2 | L1 |
| luce (bordo falda) | 2,6 → 14,7 | T3 | L2 |

## 4. LOD

Lato minore `s = min(w, h)`:

| LOD | Soglia | Parti | Raggio |
|---|---|---|---|
| L0 | s < 32 px | 2 | 0 (retto) |
| L1 | 32 ≤ s < 64 | 3 | 0 |
| L2 | s ≥ 64 px | 4 | s/8 |

Regola di gerarchia: una parte non nasce sotto la soglia del suo genitore. Semplificare è rimuovere parti, mai cambiare forma.

## 5. Vincoli e proporzioni (stile Apple)

- **Griglia 8 pt**: ogni coordinata è k/16 del lato; su un'icona da 128 pt un passo vale 8 pt.
- **Raggio icona**: s/8 (12,5%). Raggio ≤ metà del lato corto della forma.
- **Padding**: s/8 dal bordo della cella. **Spazio fra elementi**: s/16.
- **Spessore linea**: ≥ 1 px fisico; a L2 si usa s/32.
- **Palette**: 4 token. Nessun colore RGB nei giochi o nelle app.
- **Testo e finestre**: stessa unità `U = s/16` per altezza riga, raggio e padding. Il testo non ha LOD proprio: segue il lato della finestra che lo contiene.
- **Figli dentro il genitore**: ogni figlio vive nel rettangolo del genitore; le sue coordinate sono frazioni di esso.

## 6. Cache

- Chiave: `(tipo icona, w, h)`. Un slot fisso per tipo (`Cache<N>`, default 9: 3 dimensioni × 3 schermi).
- **Hit**: stessa `w`×`h`, si restituisce la voce.
- **Miss / sostituzione**: dimensione diversa (schermo più grande o più piccolo) → rigenera e sovrascrive lo slot. L'icona resta nitida a qualsiasi risoluzione.
- Invalidazione: cambio dimensione o (da fare) cambio palette.
- Voce: 120 byte (geometria relativa, nessun pixel). Bitmap RGBA 96×96 equivalente: 36 864 byte.
- Le coordinate in cache sono relative all'origine: lo spostamento della finestra non invalida nulla.

## 7. Risultati prototipo

Icona "cartella", 3 dimensioni logiche × 2 schermi (`design/screens/gen_folder_*.png`):

| Schermo | Logico 24 | Logico 48 | Logico 96 |
|---|---|---|---|
| 1920×1080 (1×) | fisico 24, **L0**, 2 prim. | fisico 48, **L1**, 3 prim. | fisico 96, **L2**, 4 prim. |
| 3840×2160 (2×) | fisico 48, **L1**, 3 prim. | fisico 96, **L2**, 4 prim. | fisico 192, **L2**, 4 prim. |

Costi (host x86, release, media su 1 000 000 ripetizioni, icona 96 px):

| Operazione | Tempo |
|---|---|
| genera da formula | ~40 ns |
| lookup in cache (hit, copia 120 B) | ~33 ns |
| disegno 4 primitivi | ~13 700 ns (*) |

(*) misurato con il canvas di prova dell'esempio, che controlla ogni pixel degli angoli. Non è il costo del backend reale: va rimisurato dopo il collegamento a `render.rs`.

Conclusione: generare costa quanto leggere la cache. La cache serve per il disegno e per non ricalcolare a ogni frame; lo spazio è 120 B contro 36 KB.

Cache: 6 miss al primo giro (3 per schermo), 6 hit al secondo giro: nessuna icona viene rigenerata.

## 8. Limiti noti

- La linguetta a L2 usa raggio pieno e risulta a "pillola". Da fare: raggio solo sugli angoli superiori.
- Un solo tipo di icona (cartella). Gli altri elementi (finestre, pulsanti, testo) seguono nella prossima fase.
- Nessuna dipendenza dal formato pixel: i colori arrivano dai token.
- Il disegno non è ancora nel percorso della scrivania.

## 9. Passi successivi

1. Raggi per angolo; poi icone file, terminale, calcolatrice con gli stessi 4 primitivi.
2. Finestre e pulsanti con la stessa unità `U`.
3. Collegare `Cache` a `render.rs` e misurare il costo sul backend reale.
4. Invalidazione su cambio palette.

Collegamenti: [vector-render-api.md](vector-render-api.md), [graphic-bible.md](graphic-bible.md), [gui-system.md](gui-system.md).
