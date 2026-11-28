# Kart 3D: piano

Obiettivo: gioco di kart 3D rudimentale, tipo TuxKart, interamente nativo HaikuOS. Nessun motore esterno, nessuna libc, Rust `no_std`, solo il nostro rasterizzatore.

## Stato

| Parte | Stato |
|---|---|
| Fisica, giri, IA, oggetti (`kart.rs`, `ai.rs`, `race.rs`) | esistente, invariata |
| Render 3D a prospettiva, punto fisso 16.16 (`render.rs`) | fatto |
| Cielo con gradiente, orizzonte, prato | fatto |
| Pista piatta con strisce e cordoli, kart a scatola, scatole oggetto, gusci e banane | fatto |
| HUD giro e posizione (glifi bitmap 3x5, non ancora vettoriali) | fatto |
| Pista come spline 3D con larghezza e banking | da fare |
| Kart a forme più ricche, ruote, sterzo visibile | da fare |
| Input da tastiera del sistema | da fare (il frame oggi usa l'IA anche per il kart 0) |
| HUD in testo vettoriale (Canvas) | da fare |

## Scelte

- Camera: dietro il kart `cam`, 60 unità indietro, 30 in alto, orizzonte a 2/5 dell'altezza, focale 2/3 dell'altezza.
- Proiezione senza float: `sx = w/2 + r·F/d`, `sy = h·2/5 - z·F/d`, con `d` profondità lungo l'asse camera.
- Piano vicino a 4 unità: i poligoni vengono tagliati (Sutherland-Hodgman su un piano).
- Painter's algorithm: pista per prima (non si sovrappone a se stessa in vista), poi oggetti ordinati per profondità del baricentro, con scarto delle facce rivolte via.
- Nessuno z-buffer, nessuna allocazione: array fissi sullo stack (`N_TRACK = 512`, `N_OBJ = 256`).

## Prossimi passi

1. Pista come spline 3D (punti di controllo, interpolazione a sottosegmenti) con larghezza e banking.
2. Input della tastiera nel frame interattivo.
3. Risolvere l'uscita di pista degli IA dopo la partenza (vedi sotto).
4. HUD con il Canvas vettoriale.

## Problemi noti

- Dal tick circa 160 gli IA escono di pista e restano sull'erba (distanza dalla linea centrale fino a ~80 unità, contro mezza larghezza 40). Il difetto è nella fisica/IA esistente, non nel render. Da indagare prima di decidere il rewrite della guida.
- Pista piatta: nessuna pendenza né banking.
- Il frame dei 5 pollici usa 800x480, quello dei 7 pollici 1920x1080 (ipotesi: 5 pollici = 800x480).

## Verifica

- `cargo test` nel crate `games/kart`: 19 test verdi.
- `kart shot W H out.ppm [tick]` scrive un fotogramma PPM; `kart bench` misura il costo.
- Screenshot in `docs/screenshots/kart-3d/`: `kart3d_7in_1920x1080.png` e `kart3d_5in_800x480.png`, tick 120 (partenza). Costo: circa 157 µs per frame 640x480 in release su host, sim e render insieme.
