//! Dati: carte e mazzi. Gli id sono indici di `DEFS`.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Row {
    Close,
    Ranged,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Unità: potenza, nessun effetto speciale.
    Unit,
    /// +1 alle altre unità della stessa fila.
    Morale,
    /// Va in fila; poi riporta in mano l'unità più forte del cimitero.
    Medic,
    /// Va nella fila dell'avversario; chi la gioca pesca 2 carte.
    Spy,
    /// Fila vicina: la potenza base diventa 1.
    Frost,
    /// Fila a distanza: la potenza base diventa 1.
    Fog,
    /// Toglie il meteo.
    Clear,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Def {
    pub name: &'static str,
    pub kind: Kind,
    pub row: Row,
    pub power: u8,
    /// Gruppo per "Schieramento": giocando una carta, si pescano le altre dello stesso gruppo. 0 = nessuno.
    pub group: u8,
}

const fn def(name: &'static str, kind: Kind, row: Row, power: u8, group: u8) -> Def {
    Def { name, kind, row, power, group }
}

use Kind::*;
use Row::*;

pub const DEFS: [Def; 23] = [
    def("Fante", Unit, Close, 2, 0),         // 0
    def("Lancere", Unit, Close, 4, 0),       // 1
    def("Guerriero", Unit, Close, 6, 0),     // 2
    def("Cavaliere", Unit, Close, 8, 0),     // 3
    def("Gigante", Unit, Close, 10, 0),      // 4
    def("Lupo grigio", Unit, Close, 3, 1),   // 5
    def("Lupo alfa", Unit, Close, 5, 1),     // 6
    def("Lupo bianco", Unit, Close, 4, 1),   // 7
    def("Guaritore", Medic, Close, 0, 0),    // 8
    def("Stendardo", Morale, Close, 2, 0),   // 9
    def("Spia", Spy, Close, 2, 0),           // 10
    def("Arciere", Unit, Ranged, 4, 2),      // 11
    def("Balestriere", Unit, Ranged, 5, 2),  // 12
    def("Arciere scelto", Unit, Ranged, 6, 2), // 13
    def("Mago", Unit, Ranged, 7, 0),         // 14
    def("Catapulta", Unit, Ranged, 8, 0),    // 15
    def("Novizio", Unit, Ranged, 1, 0),      // 16
    def("Trombettiere", Morale, Ranged, 2, 0), // 17
    def("Esploratore", Spy, Ranged, 1, 0),   // 18
    def("Guaritrice", Medic, Ranged, 0, 0),  // 19
    def("Gelo", Frost, Close, 0, 0),         // 20
    def("Nebbia", Fog, Ranged, 0, 0),        // 21
    def("Sereno", Clear, Close, 0, 0),       // 22
];

/// Mazzo del giocatore (22 carte).
pub const DECK_A: [u8; 22] = [0, 0, 1, 1, 2, 3, 4, 5, 5, 6, 7, 8, 9, 10, 11, 11, 12, 13, 14, 15, 20, 22];
/// Mazzo dell'avversario (22 carte).
pub const DECK_B: [u8; 22] = [16, 16, 0, 0, 1, 2, 2, 3, 5, 6, 7, 19, 17, 18, 11, 12, 13, 14, 21, 20, 9, 5];
