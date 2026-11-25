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

Non ancora presenti nel codice: browser, mail, mappe, musica, foto, scacchi, compressione e gli altri programmi della lista.
