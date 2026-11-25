//! Icone 16×16 (bibbia grafica, sezione 8): blocco 14×14 in T2 con segni in T0.
//! Ogni glifo è una lista di rettangoli (x, y, w, h) in coordinate di cella.
//! Selezionata: cella piena T1, stessi segni. Nessun colore fuori dai token.
pub type Cut = (i32, i32, i32, i32);

/// Blocco dell'icona, dentro la cella 16×16.
pub const BLOCK: (i32, i32, i32, i32) = (1, 1, 14, 14);

const FILE: &[Cut] = &[(4, 4, 8, 1), (4, 7, 8, 1), (4, 10, 6, 1)];
const TERM: &[Cut] = &[(3, 4, 1, 1), (4, 5, 1, 1), (5, 6, 1, 1), (4, 7, 1, 1), (3, 8, 1, 1), (7, 9, 4, 1)];
const CALC: &[Cut] = &[
    (3, 3, 2, 2), (6, 3, 2, 2), (9, 3, 2, 2),
    (3, 6, 2, 2), (6, 6, 2, 2), (9, 6, 2, 2),
    (3, 9, 2, 2), (6, 9, 2, 2), (9, 9, 2, 2),
];
const WEB: &[Cut] = &[(7, 3, 1, 10), (3, 7, 10, 1), (4, 5, 8, 1), (4, 9, 8, 1)];
const EDIT: &[Cut] = &[(3, 11, 2, 1), (4, 10, 2, 1), (5, 9, 2, 1), (6, 8, 2, 1), (7, 7, 2, 1), (8, 6, 2, 1), (9, 5, 2, 1), (10, 4, 2, 1)];
const SET: &[Cut] = &[(3, 4, 8, 1), (3, 7, 8, 1), (3, 10, 8, 1), (8, 3, 2, 3), (4, 6, 2, 3), (6, 9, 2, 3)];
const CLOCK: &[Cut] = &[(7, 4, 1, 4), (7, 7, 3, 1)];
const READ: &[Cut] = &[(7, 3, 1, 9), (3, 5, 3, 1), (3, 7, 3, 1), (3, 9, 3, 1), (9, 5, 3, 1), (9, 7, 3, 1), (9, 9, 3, 1)];
const DEFAULT: &[Cut] = &[(4, 4, 6, 6)];

/// Segni del glifo per l'etichetta dell'icona; sconosciute: riquadro vuoto.
pub fn cuts(label: &str) -> &'static [Cut] {
    match label {
        "FILE" => FILE,
        "TERM" => TERM,
        "CALC" => CALC,
        "WEB" | "NOTE" => WEB,
        "EDIT" => EDIT,
        "SET" => SET,
        "ORA" | "CLOCK" => CLOCK,
        "READ" => READ,
        _ => DEFAULT,
    }
}
