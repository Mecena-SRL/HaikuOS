# Font di sistema

Tutti sotto licenza SIL Open Font License 1.1 (`OFL.txt` in ogni cartella). Una cartella per famiglia.

| Font | File | Licenza | Uso |
|---|---|---|---|
| Inter | Inter/Inter-Variable.ttf | OFL | **UI (default sans)** |
| Noto Sans | NotoSans/NotoSans-Regular.ttf | OFL | sans alternativo |
| Source Sans 3 | SourceSans3/SourceSans3-Regular.ttf | OFL | sans alternativo |
| Noto Serif | NotoSerif/NotoSerif-Regular.ttf | OFL | **lettura (default serif)** |
| Source Serif 4 | SourceSerif4/SourceSerif4-Regular.ttf | OFL | serif alternativo |
| JetBrains Mono | JetBrainsMono/JetBrainsMono-Regular.ttf | OFL | **terminale (default mono)** |
| Noto Sans Mono | NotoSansMono/NotoSansMono-Regular.ttf | OFL | mono alternativo |
| Source Code Pro | SourceCodePro/SourceCodePro-Regular.ttf | OFL | mono alternativo |
| Space Mono | SpaceMono/SpaceMono-Regular.ttf | OFL | display / giochi |
| Atkinson Hyperlegible | AtkinsonHyperlegible/AtkinsonHyperlegible-Regular.ttf | OFL | accessibilità |

Tutti i file sono TrueType (contorni vettoriali). Default scelti in `gui/src/vfont.rs` (`default_font`); i file sono incorporati in `gui/src/fonts.rs` e letti da `gui/src/ttf.rs`.
