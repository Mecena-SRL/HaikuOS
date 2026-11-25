//! Contenuto statico delle app di sistema, una riga per voce (passo 6 px). Stato vero: fase successiva.

pub fn lines(app: u8) -> &'static [&'static str] {
    match app {
        1 => &["LEGGIMI.TXT", "DOCUMENTI", "GIOCHI"],
        2 => &["HAIKUOS 0.1", "$ _"],
        3 => &["0"],
        4 => &["BENVENUTO IN HAIKUOS", "PAGINE OFFLINE"],
        5 => &["-- SCRIPT --", "ECHO CIAO"],
        _ => &[],
    }
}

/// Passo tra due righe di testo nel corpo di una finestra.
pub const LINE_H: i32 = 6;
