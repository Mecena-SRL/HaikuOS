# Kart 3D v2: piano di riscrittura

Stato di partenza: PR #319 (render 3D, IA a scatti, test IA ignorato). Riscrittura su questo branch, #319 invariata.

## Difetti del punto di partenza (dai test)

| Area | Difetto | Prova |
|---|---|---|
| IA | sterzo on/off, non frena mai, esce di pista | `ai_stays_on_track_all_tracks` fallisce, Anello tick 57 |
| Fisica | raggio di curva fisso ~109 unità a ogni velocità | calcolo su `kart.rs` |
| Fisica | boost/stella oltre il massimo di sterzo; spin di ~2 giri | `kart.rs` righe 67, 89 |
| Pista | Serpente con bordi che si sovrappongono (~53°) | `track.rs` |
| Render | insertion sort O(n²); 48 KB di array per frame; quad scartati in silenzio oltre 512 | `render.rs` |
| HUD | glifi mancanti in silenzio; O e 0 si confondono | `render.rs` |
| Gioco | nessun loop, nessun input da tastiera, nessuna fine gara a schermo | - |
| Test | nessun test su render, IA, oggetti, muro | - |

## Obiettivi v2

1. Fisica: raggio dipendente dalla velocità, sterzo proporzionale, freno in curva, spin di un giro esatto.
2. IA: guardia in avanti sulla traiettoria, sterzo proporzionale all'errore, test "resta sulla pista" attivo su tutte le piste.
3. Pista: spline Catmull-Rom campionata una volta all'avvio, con conteggio dei campioni dimensionato.
4. Render: ordinamento O(n) per la pista, buffer riutilizzati, nessun quad scartato in silenzio (contatore esplicito).
5. HUD: set di glifi completo, O distinta da 0.
6. Gioco: loop a passo fisso con input della tastiera per il kart 0.
7. Test: uno per area (fisica, IA, muro, render, HUD, oggetti).

## Ordine di lavoro

1. Test rossi per ogni difetto della tabella.
2. Fisica (un commit).
3. IA (un commit, con test verde su tutte le piste).
4. Render (un commit).
5. Pista spline (un commit).
6. HUD e loop (un commit).
7. Screenshot 1920x1080 e 800x480, con tester Haiku.

Nessun merge senza il sì di Ivan.
