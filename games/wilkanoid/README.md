# Wilkanoid (v0)

Brick breaker in stile Arkanoid. Vettoriale, `no_std`, senza allocazione, a punto fisso.

- `src/lib.rs`: logica (racchetta, palla, mattoni, 4 livelli, 3 vite, power-up racchetta larga). Campo logico 240x135 (16:9).
- `src/render.rs`: render vettoriale (forme antialiasate, sfumature, font 3x5 a pixel). Scala a qualsiasi risoluzione.
- `src/main.rs` (host): `wilkanoid shot <cartella>` scrive i PPM, `wilkanoid sim` gioca in automatico.
- Comandi (da piattaforma): sinistra/destra per la racchetta, spazio per lanciare.

Test: `cargo test -p wilkanoid`. Il bot in `autoplay` completa tutti i livelli (test `bot_clears_all_levels`).

Screenshot in `screenshots/`: 7" 1920x1080 e 5" 1372x772 (stessa densità, ~315 PPI).

Non ancora fatto: loop in tempo reale sul framebuffer HaikuOS e backend GBA (serve un `poll_key` non bloccante nel trait `Platform`).
