# Wilkanoid v2: progetto

Pilastri:
1. Fisica leggibile: una palla, un modello di rimbalzo (cerchio contro rettangolo), velocità che cresce per livello.
2. Un meccanismo nuovo per livello: normali, acciaio (due colpi), esplosivi (rompono i vicini). Ogni livello ne introduce o combina uno.
3. Feedback: lampo sul colpo, particelle, scossa breve. Niente audio (nessun driver in `no_std`).
4. Power-up pochi e chiari: racchetta larga, multipalla.

Non è un clone: gli esplosivi cambiano il modo di pianificare i colpi.

Stato: sei livelli, il bot li completa. Restano da fare: livello di tutorial, pausa, input su framebuffer.

## File
- `src/lib.rs`: logica (fasi, racchetta, palle, mattoni, esplosivi, power-up, livelli, bot `autoplay`) e test.
- `src/render.rs`: renderer vettoriale (`render`), font 3x5, stelle, scossa.
- `src/main.rs`: host. `cargo run -q -- sim` gioca in automatico; `cargo run -q -- shot <cartella>` scrive PPM.
- Test: `cargo test`. Include il bot su 40 seed.

## Voti playtest (round 3, 15 tester)
Livelli 1-6: 5, 5, 6, 4, 6, 6. Esplosivi 4 (ora la catena si propaga). Multipalla 7, racchetta larga 7, fisica 7, grafica 6, codice 7, chiarezza 7. Complessivo 6.
Idee ancora aperte: fusibile per l'esplosivo, rampa di velocità per mattoni colpiti, tutorial, pausa, input su framebuffer.
