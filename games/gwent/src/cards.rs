//! Dati: carte e mazzi. Gli id sono indici di `DEFS`.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Row {
    Close,
    Ranged,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Unità: potenza, più eventuali bonus di fila.
    Unit,
    /// +1 a ogni unità della sua fila (non alle altre carte di fila).
    Morale,
    /// Va in fila; poi riporta in mano l'unità più forte del cimitero.
    Medic,
    /// Va nella fila dell'avversario; chi la gioca pesca 2 carte.
    Spy,
    /// Raddoppia le unità della sua fila.
    Horn,
    /// Fila vicina: la potenza base delle unità diventa 1.
    Frost,
    /// Fila a distanza: la potenza base delle unità diventa 1.
    Fog,
    /// Toglie il meteo.
    Clear,
    /// Distrugge l'unità più forte del tabellone (anche la propria).
    Scorch,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Def {
    pub name: &'static str,
    pub kind: Kind,
    pub row: Row,
    pub power: u8,
    /// Gruppo per "Schieramento": giocando una carta, si pescano le altre dello stesso gruppo. 0 = nessuno.
    pub group: u8,
    /// Effetto in una riga, mostrato al giocatore.
    pub desc: &'static str,
}

const fn def(name: &'static str, kind: Kind, row: Row, power: u8, group: u8, desc: &'static str) -> Def {
    Def { name, kind, row, power, group, desc }
}

use Kind::*;
use Row::*;

pub const DEFS: [Def; 25] = [
    def("Fante", Unit, Close, 2, 0, "Solo potenza."),                                        // 0
    def("Lancere", Unit, Close, 4, 0, "Solo potenza."),                                      // 1
    def("Guerriero", Unit, Close, 6, 0, "Solo potenza."),                                    // 2
    def("Cavaliere", Unit, Close, 8, 0, "Solo potenza."),                                    // 3
    def("Gigante", Unit, Close, 10, 0, "Solo potenza."),                                     // 4
    def("Lupo grigio", Unit, Close, 3, 1, "Schieramento: pesca dal mazzo gli altri Lupi."),  // 5
    def("Lupo alfa", Unit, Close, 5, 1, "Schieramento: pesca dal mazzo gli altri Lupi."),    // 6
    def("Lupo bianco", Unit, Close, 4, 1, "Schieramento: pesca dal mazzo gli altri Lupi."),  // 7
    def("Guaritore", Medic, Close, 0, 0, "Riporta in mano l'unita piu forte del cimitero."), // 8
    def("Stendardo", Morale, Close, 2, 0, "Morale: +1 a ogni unita della sua fila."),        // 9
    def("Spia", Spy, Close, 2, 0, "Va nella fila avversaria. Pesca 2 carte. Punti a lui."),   // 10
    def("Arciere", Unit, Ranged, 4, 2, "Schieramento: pesca dal mazzo gli altri Arcieri."),  // 11
    def("Balestriere", Unit, Ranged, 5, 2, "Schieramento: pesca dal mazzo gli altri Arcieri."), // 12
    def("Arciere scelto", Unit, Ranged, 6, 2, "Schieramento: pesca dal mazzo gli altri Arcieri."), // 13
    def("Mago", Unit, Ranged, 7, 0, "Solo potenza."),                                        // 14
    def("Catapulta", Unit, Ranged, 8, 0, "Solo potenza."),                                   // 15
    def("Novizio", Unit, Ranged, 1, 0, "Solo potenza."),                                     // 16
    def("Trombettiere", Morale, Ranged, 2, 0, "Morale: +1 a ogni unita della sua fila."),    // 17
    def("Esploratore", Spy, Ranged, 1, 0, "Va nella fila avversaria. Pesca 2 carte. Punti a lui."), // 18
    def("Guaritrice", Medic, Ranged, 0, 0, "Riporta in mano l'unita piu forte del cimitero."), // 19
    def("Gelo", Frost, Close, 0, 0, "Meteo: la fila vicina scende a potenza 1."),           // 20
    def("Nebbia", Fog, Ranged, 0, 0, "Meteo: la fila a distanza scende a potenza 1."),      // 21
    def("Sereno", Clear, Close, 0, 0, "Toglie ogni meteo."),                                 // 22
    def("Corno", Horn, Close, 0, 0, "Raddoppia le unita della sua fila."),                  // 23
    def("Scorch", Scorch, Close, 0, 0, "Distrugge l'unita piu forte del tabellone."),        // 24
];

/// Mazzo del giocatore (22 carte).
pub const DECK_A: [u8; 22] = [0, 23, 1, 1, 2, 3, 4, 5, 5, 6, 7, 8, 9, 10, 11, 24, 12, 13, 14, 15, 20, 22];
/// Mazzo dell'avversario (22 carte).
pub const DECK_B: [u8; 22] = [16, 23, 18, 0, 1, 2, 2, 3, 5, 6, 7, 19, 17, 18, 11, 12, 13, 14, 21, 20, 9, 5];
