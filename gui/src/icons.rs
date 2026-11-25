//! Icone 16×16 (bibbia grafica, sezione 8): blocco 14×14 in T2 con segni in T0.
//! Ogni glifo è una lista di rettangoli (x, y, w, h) in coordinate di cella.
//! Selezionata: cella piena T1, stessi segni. Nessun colore fuori dai token.
use crate::palette::Tok;
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
const REMIND: &[Cut] = &[(7, 3, 2, 1), (5, 4, 6, 5), (4, 9, 8, 1), (7, 10, 2, 2)];
const PASSWORDS: &[Cut] = &[(3, 6, 3, 4), (6, 7, 6, 2), (10, 9, 1, 2), (12, 9, 1, 2)];
const DICTIONARY: &[Cut] = &[(7, 4, 1, 8), (4, 4, 3, 1), (9, 4, 3, 1), (4, 6, 3, 1), (9, 6, 3, 1), (4, 8, 3, 1), (9, 8, 3, 1)];
const CALENDAR: &[Cut] = &[(3, 4, 10, 1), (3, 4, 1, 9), (12, 4, 1, 9), (3, 12, 10, 1), (3, 7, 10, 1), (6, 3, 1, 2), (9, 3, 1, 2)];
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
        "REMIND" => REMIND,
        "PASSWORDS" => PASSWORDS,
        "DICTIONARY" => DICTIONARY,
        "CALENDAR" => CALENDAR,
        _ => DEFAULT,
    }
}

// ---- Icone vere x86 (bibbia §8, 32×32 px fisici). Rettangoli (x, y, w, h, token) su griglia 32.
// Il piatto colorato (accento dell'app) lo disegna il canvas; qui solo i segni.
pub type Shape = (i32, i32, i32, i32, Tok);
use crate::palette::Tok::{T0, T1};

const A_FILE: &[Shape] = &[(6, 8, 10, 4, T0), (6, 11, 20, 14, T0), (8, 13, 16, 1, T1)];
const A_TERM: &[Shape] = &[(5, 7, 22, 18, T0), (8, 11, 2, 2, T1), (10, 13, 2, 2, T1), (8, 15, 2, 2, T1), (13, 18, 8, 2, T1)];
const A_CALC: &[Shape] = &[
    (8, 5, 16, 22, T0), (10, 7, 12, 5, T1),
    (10, 14, 3, 3, T1), (14, 14, 3, 3, T1), (18, 14, 3, 3, T1),
    (10, 19, 3, 3, T1), (14, 19, 3, 3, T1), (18, 19, 3, 3, T1),
    (10, 24, 3, 2, T1), (14, 24, 3, 2, T1), (18, 24, 3, 2, T1),
];
const A_WEB: &[Shape] = &[(6, 6, 20, 20, T0), (15, 6, 2, 20, T1), (6, 15, 20, 2, T1), (9, 10, 14, 1, T1), (9, 21, 14, 1, T1)];
const A_EDIT: &[Shape] = &[
    (8, 24, 3, 3, T1), (11, 21, 4, 4, T0), (15, 17, 4, 4, T0), (19, 13, 4, 4, T0), (23, 9, 3, 3, T0),
];
const A_SET: &[Shape] = &[
    (6, 9, 20, 2, T0), (6, 15, 20, 2, T0), (6, 21, 20, 2, T0),
    (18, 7, 4, 6, T1), (9, 13, 4, 6, T1), (14, 19, 4, 6, T1),
];
const A_CLOCK: &[Shape] = &[(6, 6, 20, 20, T0), (15, 10, 2, 8, T1), (15, 16, 7, 2, T1)];
const A_READ: &[Shape] = &[
    (6, 8, 11, 16, T1), (17, 8, 9, 16, T1), (15, 8, 2, 16, T0),
    (8, 12, 6, 2, T0), (8, 16, 6, 2, T0), (19, 12, 5, 2, T0), (19, 16, 5, 2, T0),
];
const A_DEFAULT: &[Shape] = &[(10, 10, 12, 12, T1)];

/// Segni dell'icona vera per l'etichetta; sconosciute: un quadrato.
pub fn art(label: &str) -> &'static [Shape] {
    match label {
        "FILE" => A_FILE,
        "TERM" => A_TERM,
        "CALC" => A_CALC,
        "WEB" | "NOTE" => A_WEB,
        "EDIT" => A_EDIT,
        "SET" => A_SET,
        "ORA" | "CLOCK" => A_CLOCK,
        "READ" => A_READ,
        _ => A_DEFAULT,
    }
}
