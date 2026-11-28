# Editor di giochi (v0)

- `editor/` (`haiku-editor`): motore dati `no_std`, senza allocazione. Griglia di tile, scena con sprite, undo a pila fissa, salvataggio binario (`HGS1`).
- `editor/src/main.rs`: CLI host. Esempio: `haiku-editor scena.hgs new`, poi `paint X Y V`, `fill X0 Y0 X1 Y1 V`, `sprite KIND X Y`, `show`.
- `games/chess/` (`chess`): regole degli scacchi `no_std` sulla `Grid` dell'editor. Mancano arrocco ed en passant; la promozione è sempre a donna. Test: perft iniziale (20/400/8902).
- `games/rpg/` (`rpg`): RPG stile giallo `no_std`: mappa a tile (Verdepoli, Percorso 1), NPC con dialoghi, battaglia a turni con tipi, cattura, allenatore CPU. Gioco da terminale (`cargo run -p rpg`, w/a/s/d, z, x) e screenshot (`rpg shot`). Test: `cargo test -p rpg`.

Test: `cargo test --workspace`. Gioco: `cargo run -p chess`.

Prossimi passi: interfaccia grafica sul framebuffer, formato sprite con palette, arrocco/en passant, backend GBA.

## GUI (v1)
- `editor/src/gui.rs`: canvas `u32` ARGB, stato `Editor` (cursore, pennello, undo), tasti logici `Key`, render scena. `no_std`.
- Anteprima host: `haiku-editor scena.hgs render out.ppm`.
- Driver framebuffer di HaikuOS: da fare quando esiste il livello grafico.

## Piattaforma (v2)
- `editor/src/platform.rs`: trait `Platform` (dimensioni, buffer, tasti, present) e `run()`. Ogni backend lo implementa: host (`edit`), framebuffer HaikuOS, GBA.
- Il trait ha un tipo pixel associato: `u32` ARGB sull'host, `u16` BGR555 sul GBA (Mode 3 a 240x160, senza copie).
- Scena v2 (`HGS2`): tile + livello solido (collisioni, tasto `x` nell'edit). Il solido non è annullabile con undo (da fare).
