# RPG: decision record (rewrite)

Status: decided, not yet implemented. Source: 45-expert brainstorm plus lead decision (182 proposals). Grounded in `src/battle.rs`, `src/lib.rs`, `src/world.rs`. `draw.rs` and `main.rs` were not reviewed for this record.

## Scope

- IN: 3 maps (16x12 tiles each), 4 species, 8 moves, party of 3, wild battles, 2 trainers, one nurse, one save slot, a Start party screen, terminal play and PPM shots on host, GBA target.
- IN: one XP and level system (levels 1 to 10) shared by wild and trainer fights.
- IN: a switch action in battle (CAMBIA), an accuracy field per move, a per-species catch value.
- OUT: PP, status conditions, STAB, critical hits, damage spread, Guard, happiness, evolution, money, shop, bag or potions, PC box, follower, starter choice, title menu, walk-up trainer animation, typewriter text, HP drain animation, speed-based flee, hold-to-repeat walking, 2x2 command grid, per-patch encounter tables, sound.
- Revisit OUT items only after the IN list passes transcript tests.

## Maps

- Three maps: 0 VERDEPOLI (town), 1 PERCORSO 1, 2 PERCORSO 2. Each 16x12 tiles.
- Tile 8x8 px. Map 128x96 px, 192 cells, ASCII source packed at build time.
- Warps (both directions, data table): V(15,6) to P1(1,6); P1(0,6) to V(14,6); P1(15,6) to P2(1,6), locked unless flags bit 0; P2(0,6) to P1(14,6).
- Locked `>` on P1 shows "SENTIERO CHIUSO" and does not move the player.
- VERDEPOLI: extend the `=` path on row 6 from x=5 to x=14 to reach the gate (data only).
- VERDEPOLI sign NPC at (13,5): "A EST: PERCORSO 1|E L'ALLENATORE".
- VERDEPOLI nurse at (2,2). START (7,8).
- PERCORSO 1 trainer 1 at (9,5), facing West. PERCORSO 2 trainer 2 at (9,5), facing West; authored in milestone 3 with a closed border.
- Grass `,` is the only encounter tile. VERDEPOLI has none.

## Party and storage

- Party max 3. Slot order is catch order. `active` indexes the party.
- Start: n=1, FLAMEL level 5 in slot 0, balls=5, flags=0.
- Catch with n=3 refused: "SQUADRA PIENA!". No ball spent, no turn.
- CAMBIA lists living members, costs the turn; the foe's attack lands on the incoming creature.
- After a faint, the player picks the next creature from a list (no auto-pick). None alive means the battle is lost.
- No storage, no release, no reordering outside battle.
- Lose: heal all, `active=0`, warp to map 0 START, keep levels and XP.

## Battle

- Main menu: LOTTA, CAMBIA, CATTURA, FUGGI. Cursor clamped 0..=3, no wrap.
- Moves submenu: 2 moves, clamped 0..=1. B returns to main menu.
- Hit if `battle_rng.below(100) < accuracy`; miss prints "MANCATO!".
- Damage: `dmg = max(1, (atk * power / (def + 4)) * eff / 100)`, atk and def from `stat(base, lv)`.
- Effectiveness 200, 100, 50. Print "E' SUPER EFFICACE!" on 200, "NON MOLTO EFFICACE." on 50.
- Turn order: higher `spd` first; tie goes to player.
- Foe move, drawn once at turn start: with `battle_rng.below(100) < 70` pick the highest-damage move (ties to slot 0), else `battle_rng.below(2)`.
- Wild RUN always succeeds, no turn spent. Trainer RUN and trainer CATTURA refused, no turn spent.
- Moves (index, name, kind, power, accuracy): 0 EMBER Fuoco 10/95; 1 BOLLA Acqua 10/95; 2 FRUSTA Erba 10/95; 3 PLACCA Normale 8/100; 4 GETTO Acqua 12/75; 5 SPORA Erba 12/75; 6 FIAMMATA Fuoco 14/75; 7 MORSO Normale 12/80.
- Species moves: FLAMEL [0,6], AQUO [1,4], LEAFO [2,5], RATTO [3,7].
- Messages: a page is 2 lines of at most 26 chars. A turn emits at most 6 pages, fixed buffer 6 x 2 x 26 bytes. Overflow is a test failure, never a silent drop.

## Creatures

- Species table append-only. 0 FLAMEL (Fuoco), 1 AQUO (Acqua), 2 LEAFO (Erba), 3 RATTO (Normale).
- Base hp/atk/def/spd: FLAMEL 22/9/6/8, AQUO 20/6/7/6, LEAFO 23/8/7/7, RATTO 14/6/4/9.
- Catch value: FLAMEL 60, AQUO 120, LEAFO 90, RATTO 180.
- Stats, integer only: `stat(base, lv) = base * (lv + 9) / 10` (lv 1 = base, lv 5 = 1.4x, lv 10 = 1.9x). Applies to hp, atk, def, spd.
- Record: `sp: u8`, `lv: u8`, `xp: u16`, `hp: u8` (current). Max HP = `stat(base_hp, lv)`.
- Check: FLAMEL EMBER into LEAFO lv5 = `12*10/(9+4)=9`, x2 = 18.

## Progression

- Total XP for level L is L^3 (L2=8 ... L10=1000). Cap 10.
- Each won battle gives each living member `foe_lv * 4` XP.
- Level-up raises max HP by the stat delta and adds the same delta to current HP. Print "X SALE AL LIVELLO N!" per level.
- Wild level `2 + battle_rng.below(3)` (2 to 4). Starter FLAMEL lv 5. No evolution.

## Trainers and story

- `flags: u8`: bit 0 trainer 1 beaten (gates PERCORSO 2); bit 1 trainer 2 beaten; bit 2 intro seen.
- Trainer 1 team: RATTO lv4, LEAFO lv5. Trainer 2 team: AQUO lv6, FLAMEL lv6, LEAFO lv7.
- Sight trigger: after each step, an unbeaten trainer that sees the player in its row within 3 tiles in its facing direction starts its intro dialogue; battle starts when it ends. A is the fallback.
- Trainer sends its next creature for free ("L'ALLENATORE MANDA X!"), no turn lost by the player.
- Trainer 1 win: bit 0, +3 balls, "HAI VINTO!|RICEVI 3 PALLE!". Trainer 2 win: bit 1, "HAI VINTO!|FINE DELLA DEMO!". Stay in Walk.
- Beaten trainer text: "BEL COMBATTIMENTO."
- First boot (bit 2 clear): "BENVENUTO! CAMMINA CON LE FRECCE.|A PARLA E CONFERMA."

## Items and healing

- No items, money, or potions.
- Nurse (2,2) on map 0: heals all, sets balls to `max(balls, 5)`, writes the save. Text "LE TUE CREATURE|SONO GUARITE!".
- Balls start at 5; +3 per trainer 1 win.

## Catching

- Wild only. `rate = min(95, (3*max - 2*hp) * catch_value / (3*max))`, integer.
- RATTO lv3 (max 16) full HP gives 60%; at 1 HP caps at 95%.
- Checks in order: party full (refuse, no ball), no balls (refuse, no ball), spend one ball. Roll `battle_rng.below(100) < rate`.
- Success "CATTURATO!" (joins at current level and HP). Failure "CATTURA NON RIUSCITA!" and the foe attacks.
- Encounters: `world_rng.below(8) == 0` per grass step, 3-step cooldown after a battle.
- Tables: PERCORSO 1 RATTO 45, LEAFO 35, AQUO 20. PERCORSO 2 AQUO 40, LEAFO 40, RATTO 20. VERDEPOLI none.

## Save

- One slot, 32 bytes, little-endian, explicit codec (no serde, no transmute).
- Magic `RP`, version 1, header (map, x, y, dir, n, active, balls, flags: u8 each), world RNG u32, party 3 x (`sp u8, lv u8, xp u16 LE, hp u8`), CRC-16/CCITT-FALSE over bytes 0..30 stored LE at 30..32.
- Written only at the nurse, in Walk with no pending trainer.
- Boot: load only if magic, version, CRC and ranges pass (map < 3, coords in bounds, n 1..=3, active < n, sp < 4, hp <= max, balls <= 99). Else new game. Loading starts in Walk at the saved tile.
- Not saved: battle state, mode, fade, `battle_rng` (re-seeded from `world_rng.next() | 1` at each battle start).
- Torn write falls back to new game (accepted for v1).

## Controls

- Walk: D-pad moves one tile and faces that way. A talks, interacts, or starts a trainer. B nothing. Start opens the party screen.
- Dialog: A or B advances a page; after the last page the box closes.
- Menus: Up/Down clamp, no wrap. A confirms, B goes back. Party screen: Up/Down scroll, B closes.
- One `update()` per frame at 60 Hz; each press is one action; no hold-repeat in v1.
- Input lockout: 8 frames after any screen change.
- Host keys: WASD or arrows, z = A, x = B, Enter or p = Start, q quit. Raw terminal mode on host only.

## Screen layout

- Target 240x160; host preview 2x (480x320) on the same 8 px grid.
- Map 128x96 at x=56, y=0. Bottom box y=120..160, full width, 2 lines, 12 px step, text x=8, 8 px glyph advance, max 26 chars per line. Page marker triangle at x=224, y=148.
- Battle top band (y 0..56): foe panel x=8..112, y=8..32 (name, Lv, HP bar, no numbers); foe sprite x=176..208, y=16..48.
- Battle lower band (y 56..120): player sprite x=32..64, y=72..104; player panel x=72..144, y=64..88 (name, Lv, HP bar, "PS cur/max").
- Command/move overlay x=152..240, y=56..116; cursor x=156, labels from x=168, max 8 chars. Overlay never covers messages.
- HP colour: green >50%, yellow >20%, red otherwise. Numbers always shown for the player.
- Each kind has a 3 px inner mark (triangle Fuoco, wave Acqua, leaf notch Erba, plain Normale), so kind is not hue-only. Creature sprites get a 1 px INK outline.
- No text past x=232 or y=160.

## Tests to write

1. All 3 maps 16x12 with closed border.
2. Warps reciprocal, land on walkable tiles; all NPCs and START walkable.
3. P1 east `>` refused without flag bit 0, allowed with it.
4. Every dialogue page and battle line fits 2x26; every character has a glyph (missing glyph fails).
5. `stat(base, 5)` matches the table for all species; XP thresholds match L^3.
6. Damage: FLAMEL EMBER into LEAFO L5 = 18; into AQUO L5 = 4.
7. Hit rate over 10000 rolls within 2 points of accuracy, per move.
8. Catch RATTO L3 full HP = 60; at 1 HP = 95. Full party / no balls refuse without spending ball or turn.
9. Trainer sends next creature free; trainer catch and run refused.
10. Trainer 1 win sets bit 0, +3 balls, unlocks PERCORSO 2.
11. Level-up raises max and current HP by the delta.
12. Lose path: healed, `active=0`, map 0 START, levels and XP kept.
13. Save round trip keeps every field; bad CRC, magic, or all-0xFF boot fall back to new game.
14. Cursor clamp at both ends.
15. Property sweep seeds 1..=500 with random legal keys: no panic, no overflow, HP >= 0, `active < n`.
16. Two-stream determinism: changing a battle formula does not change the first 20 encounter steps for a seed.
17. Replay transcripts: trainer 1 win from seed 7, full loss, catch to full party.
18. Golden FNV-1a hashes of frames at 240x160 for four scenes.
19. `size_of::<Game>() <= 512` compile-time assert.

## Implementation plan

1. Fix known defects in current code: `Msg::byte` silently drops bytes past 128; RATTO moves `[3, 3]`; the 6-line test bound (wrong for 2-line pages); check `glyph()` for silent blanks.
2. Data tables: species (u8 ids), 8 moves with accuracy, 3 maps with warps and gate, NPC table with teams, encounter tables; `const` asserts on sizes.
3. Creature model: `stat()`, `lv`, `xp`, `hp: u8`; port `damage()`, `eff()`, `heal()`.
4. Battle core: accuracy roll, turn order, effectiveness line, 2-line page queue, `world_rng` and `battle_rng` split.
5. Party: CAMBIA, forced-choice list after faint, clamped cursors.
6. Catch: new formula and refusal order.
7. Trainers: teams, free sends, sight trigger, flags, rewards, gate.
8. XP and level-up.
9. Encounter tables and cooldown.
10. Save codec, nurse write, boot load.
11. Input: `update()` per frame, lockout, Start party screen, host raw mode.
12. Layout on 240x160 grid, golden frames, CI (no `.github` directory yet).
13. Replay transcripts and property sweep; tune trainer numbers only after transcripts pass.

Balance numbers are estimates from the formulas; verify with transcripts before tuning.
