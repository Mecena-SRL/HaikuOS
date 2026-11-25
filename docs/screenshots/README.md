# Screenshot GBA

Questi PNG sono render dalla VRAM (mode 3, 240x160), letta con libmgba da `gba/host/render_shots.c` e ingranditi 3x con pixel netti (`gba/host/render_shots.py`). Non sono screenshot dello schermo reale.

Rigenerazione:

```sh
cd gba
cargo build --release
/usr/bin/llvm-objcopy -O binary target/thumbv4t-none-eabi/release/haiku-gba haiku-gba.gba
gcc -O1 -o render_shots host/render_shots.c -lmgba
./render_shots haiku-gba.gba /tmp/raw     # serve una cartella raw vuota
python3 host/render_shots.py /tmp/raw ../docs/screenshots
```

## Schermate

- `01_desktop_lanciatore.png`: desktop con la griglia delle icone (FILES, TERM, CALC, NOTES). È il lanciatore.
- `02_calcolatrice.png`: calcolatrice aperta in una finestra sul desktop.
- `03_spazio_di_lavoro.png`: spazio di lavoro sulla radice utente: directory `docs` (blu, selezionata) e file `bench` (arancio).
- `04_visualizzatore_testo.png`: visualizzatore di testo con il file `docs/hello`.
- `05_editor.png`: editor sullo stesso file, con caratteri in corso di inserimento (arancio) e testo salvato (verde).

- `06_desktop_otto_icone.png`: desktop con otto icone su due righe. In seconda riga ci sono CLOCK, SETTINGS, CHESS e READ.
- `07_scacchi_posizione_iniziale.png`: scacchi in finestra, posizione iniziale. Cursore su e2, Bianco al tratto (W).
- `08_lettore_pagina_1.png`: lettore di testo, pagina 1 di 2.

Per 06-08 usare `gba/host/render_media.c` (stessa procedura, sequenza di tasti diversa). Nota: in questa cattura i tasti non arrivano alle finestre di scacchi e lettore (vedi PR), quindi non c'è una pagina 2 né una mossa.

Non ancora presenti nel codice: browser, mail, mappe, musica, foto, compressione e gli altri programmi della lista.
