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

## 5. Unità `u` e classi di schermo

- **Superficie**: lato minore `S = min(w, h)` in pixel fisici.
- **Scala utente** `pct` (come Mac/Windows): 100 %, 125 %, 150 % (`genicon::USER_SCALES`). Moltiplica `u`.
- **Unità**: `u = max(1, round(S · pct / 12800))`, arrotondata all'intero più vicino. Su schermi piccoli `u` è un numero intero di pixel (GBA 240×160 → 1 px; PSP 480×272 → 2 px). Su schermi grandi cresce con la superficie (1080p → 8 px; 4K → 16 px).
- **Icona** = 16 u (128 px su 1080p, 32 px su 480×272). **Padding** = 2 u (s/8). **Spazio** = 1 u (s/16). **Raggio icona** = 2 u.
- **Testo** = 2 u, con minimo per classe.

| Scala | 100 % | 125 % | 150 % |
|---|---|---|---|
| 480×272: u / icona | 2 / 32 px | 3 / 48 px | 3 / 48 px |
| 1920×1080: u / icona | 8 / 128 px | 11 / 176 px | 13 / 208 px |

Con u intero a 272 px di lato, 125 % e 150 % danno lo stesso u. Per distinguerli servirebbe u in quarti di pixel (passo da pianificare).

| Classe | Lato minore | Testo minimo | Esempio |
|---|---|---|---|
| Small | < 360 px | 8 px | GBA, PSP 480×272 |
| Medium | 360–999 px | 10 px | 720p |
| Large | ≥ 1000 px | 12 px | 1080p, 4K |

Tutte le misure di font, spazi e raggi sono in `u`. Nessun valore fisso in pixel nelle app.

## 6. Vincoli e proporzioni (stile Apple)

- **Griglia**: ogni coordinata è k/16 dell'icona (= 1 u per passo).
- **Raggio icona**: 2 u (s/8). Raggio ≤ metà del lato corto della forma.
- **Spessore linea**: ≥ 1 px fisico; a L2 si usa s/32.
- **Palette**: 4 token. Nessun colore RGB nei giochi o nelle app.
- **Testo e finestre**: stessa unità `u` per altezza riga, raggio e padding. Il testo non ha LOD proprio: segue il lato della finestra che lo contiene.
- **Figli dentro il genitore**: ogni figlio vive nel rettangolo del genitore; le sue coordinate sono frazioni di esso.

## 7. Cache

- Chiave: `(tipo icona, w, h)`. Un slot fisso per tipo (`Cache<N>`, default 9: 3 dimensioni × 3 schermi).
- **Hit**: stessa `w`×`h`, si restituisce la voce.
- **Miss / sostituzione**: dimensione diversa (schermo più grande o più piccolo) → rigenera e sovrascrive lo slot. L'icona resta nitida a qualsiasi risoluzione.
- Invalidazione: cambio dimensione o (da fare) cambio palette.
- Voce: 120 byte (geometria relativa, nessun pixel). Bitmap RGBA 96×96 equivalente: 36 864 byte.
- Le coordinate in cache sono relative all'origine: lo spostamento della finestra non invalida nulla.

## 8. Risultati prototipo

Icona "cartella" con unità `u` (`design/screens/gen_folder_*.png`):

| Schermo | u | Classe | Icona | LOD | Parti (w×h@raggio) | Testo | Padding | Spazio |
|---|---|---|---|---|---|---|---|---|
| 480×272 | 2 | Small | 32 px | L1 | 28×18@0, 12×6@0, 28×16@0 | 8 px | 4 px | 2 px |
| 1920×1080 | 8 | Large | 128 px | L2 | 112×72@16, 48×24@12, 112×64@16, 96×8@0 | 16 px | 16 px | 8 px |

Versione precedente a dimensioni fisse (3 dimensioni × 1080p/4K, non più rigenerata; PNG rimosso):

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

## 9. Limiti noti

- La linguetta a L2 usa raggio pieno e risulta a "pillola". Da fare: raggio solo sugli angoli superiori.
- Un solo tipo di icona (cartella). Gli altri elementi (finestre, pulsanti, testo) seguono nella prossima fase.
- Nessuna dipendenza dal formato pixel: i colori arrivano dai token.
- Il disegno non è ancora nel percorso della scrivania.

## 10. Passi successivi

1. Raggi per angolo; poi icone file, terminale, calcolatrice con gli stessi 4 primitivi.
2. Finestre e pulsanti con la stessa unità `U`.
3. Collegare `Cache` a `render.rs` e misurare il costo sul backend reale.
4. Invalidazione su cambio palette.

Collegamenti: [vector-render-api.md](vector-render-api.md), [graphic-bible.md](graphic-bible.md), [gui-system.md](gui-system.md).
