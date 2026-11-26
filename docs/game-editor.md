# Editor di giochi (v0)

- `editor/` (`haiku-editor`): motore dati `no_std`, senza allocazione. Griglia di tile, scena con sprite, undo a pila fissa, salvataggio binario (`HGS1`).
- `editor/src/main.rs`: CLI host. Esempio: `haiku-editor scena.hgs new`, poi `paint X Y V`, `fill X0 Y0 X1 Y1 V`, `sprite KIND X Y`, `show`.
- `games/chess/` (`chess`): regole degli scacchi `no_std` sulla `Grid` dell'editor. Mancano arrocco ed en passant; la promozione è sempre a donna. Test: perft iniziale (20/400/8902).

Test: `cargo test --workspace`. Gioco: `cargo run -p chess`.

Prossimi passi: interfaccia grafica sul framebuffer, formato sprite con palette, arrocco/en passant, backend GBA.
