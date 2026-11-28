# Engine HaikuOS (`haiku-game`)

Stato: survey fatto, core `no_std` creato, pinball portato come prova. Il resto è pianificato sotto.

## 1. Duplicazioni nei giochi

Survey di `main` e dei rami delle PR aperte (#319 kart, #320 gwent, #321 rpg, #322 pinball, #323 citybuilder, #324 wilkanoid, #325 platformer). Le carte non sono un crate separato: stanno in `games/gwent/src/cards.rs`.

| Area | Dove è duplicata | Stato |
|---|---|---|
| Radice intera | pinball `isqrt(i64)`, kart `fx.rs isqrt(u64)` | Unificata: `math::isqrt` / `isqrt_i64`. Pinball portato |
| Virgola fissa | pinball 8.8 (`FP=256`), kart 16.16 (`fx.rs`), wilkanoid (#324) a punto fisso | Da fare: tipo `Fixed` a scala scelta per gioco. Tabelle seno: pinball e kart |
| Generatore casuale | kart `race.rs` xorshift32, citybuilder xorshift64, wilkanoid `rand`, pinball test con LCG | `rng::Rng` (xorshift32). Pinball portato (test). Gli altri da portare |
| Passo fisso | kart, platformer, citybuilder misurano con `Instant`; pinball conta frame | `clock::FixedStep` pronto, nessun gioco ancora lo usa |
| Menu e schermate | `on_menu_key` + enum `Next` in pinball, platformer, citybuilder | `scene::Next<S>` generico. Pinball portato. L'enum `Screen` resta per gioco |
| PPM di uscita | pinball `screen.rs`, platformer `main.rs`, citybuilder `view.rs`; `hd_shot.rs` via `#[path]` | `ppm::write_scaled` (feature `host`). Pinball portato. `hd_shot::write_fit` da migrare |
| Disegno (rect, fill, testo) | `Img` (pinball), `Frame` (citybuilder), `Fb` (platformer), `rect` (kart, wilkanoid) | Da fare: strato render sopra `haiku_gui::render::Canvas` |
| Input | `Input` (pinball, kart), bitmask `u8` (platformer), stringhe tasto (citybuilder) | Da fare: `Button` a bitmask per piattaforma |
| Bot / IA | pinball `main.rs`, platformer `bot.rs`, kart `ai.rs` | Non è motore: resta per gioco |

## 2. Design

- **`haiku-game`** (`games/haiku-game`): core `no_std`, senza allocazione. Moduli: `math`, `rng`, `clock`, `scene`. Feature `host` (std) per `ppm`.
- **Renderer**: solo vettoriale, sopra il trait `Canvas` di `haiku-gui` che già esiste. Da fare nella fase successiva.
- **Loop**: logica a 60 Hz con `FixedStep` (tick limitati per chiamata, niente spirale di ritardi). Il disegno gira a parte.
- **Input**: `Button` a bitmask, mappato per piattaforma (GBA, host, x86). Da fare.
- **Schermate**: `Next<S>` per ora. Stack a profondità fissa da fare.

### Budget per fotogramma (60 Hz = 16,67 ms)

| Fase | 1920×1080 | 800×480 |
|---|---|---|
| Logica | ≤ 1 ms | ≤ 1 ms |
| Disegno | ≤ 8 ms | ≤ 3 ms |
| Blit, input, resto | ≤ 2 ms | ≤ 1 ms |
| Margine | ≥ 5 ms | ≥ 11 ms |

Misure attuali, dalle PR (non rimisurate, tranne pinball):
- Pinball logica: 113 ns/frame (misurato qui, `pinball 600`).
- Citybuilder disegno 1920×1080: circa 17 ms, non ottimizzato, sopra budget (#323).
- Platformer logica più disegno 800×480: circa 2,7 ms, dentro budget (#325).

## 3. Prova: pinball portato

Pinball usa ora `math::isqrt_i64`, `scene::Next`, `ppm::write_scaled` e `rng::Rng` nel test.

- `cargo test --release` in `games/pinball`: verde (13 core, 5 host).
- `--shot-title`, `--shot-play`, `--shot-pause` a 1920×1080: byte per byte identici a `main` (`cmp`). La logica non cambia.
- Screenshot: `games/pinball/screens/pinball-{title,play,pause}-{1920x1080,800x480}.png`.

## 4. Ordine di porting (proposto, non eseguito)

1. **kart**: ha già l'`isqrt` identico e il suo `fx.rs` diventa il tipo `Fixed`. Rischio basso.
2. **platformer**: passo fisso, `Button`, strato render. Ha già il budget misurato.
3. **citybuilder**: unificare l'RNG e portare il disegno sotto budget (#323).
4. **wilkanoid** (#324) e **gwent** (#320), **rpg** (#321): dopo il merge delle PR.

## 5. Fuori da questo PR

Strato render, `Button`, stack di schermate, tipo `Fixed`, tabella seno condivisa. Ciascuno è un passo a sé, dopo la prova su pinball.
