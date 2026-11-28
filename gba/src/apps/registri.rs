//! Registri: ultime voci di log in un anello fisso (8 voci, senza allocazioni).
//! Logica pura; riusa `sysutil::Line`.

use super::sysutil::{Line, Out};

/// Voci conservate: la più vecchia viene scartata.
pub const VOCI: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Livello {
    Info,
    Avviso,
    Errore,
}

#[derive(Clone, Copy)]
pub struct Registri {
    voci: [Option<(Livello, &'static [u8])>; VOCI],
    prossima: usize,
    totale: usize,
    errori: usize,
}

impl Registri {
    pub const fn new() -> Self {
        Registri { voci: [None; VOCI], prossima: 0, totale: 0, errori: 0 }
    }

    pub fn push(&mut self, livello: Livello, msg: &'static [u8]) {
        self.voci[self.prossima] = Some((livello, msg));
        self.prossima = (self.prossima + 1) % VOCI;
        self.totale += 1;
        if livello == Livello::Errore {
            self.errori += 1;
        }
    }

    /// Voce `i`, dalla più recente (0) alla più vecchia.
    pub fn voce(&self, i: usize) -> Option<(Livello, &'static [u8])> {
        if i >= VOCI {
            return None;
        }
        self.voci[(self.prossima + VOCI - 1 - i) % VOCI]
    }

    /// Riga `i` per la schermata: etichetta di livello e testo.
    pub fn riga(&self, i: usize) -> Line {
        match self.voce(i) {
            None => Line::new(),
            Some((l, m)) => {
                let tag: &[u8] = match l {
                    Livello::Info => b"I ",
                    Livello::Avviso => b"A ",
                    Livello::Errore => b"E ",
                };
                Line::new().push(tag).push(m)
            }
        }
    }

    /// Riepilogo: voci totali ed errori.
    pub fn out(&self) -> Out {
        (Line::new().push(b"LOG ").num(self.totale), Line::new().push(b"ERRORI ").num(self.errori))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ultima_voce_per_prima() {
        let mut r = Registri::new();
        r.push(Livello::Info, b"avvio");
        r.push(Livello::Errore, b"disco");
        assert_eq!(r.riga(0).as_bytes(), b"E disco");
        assert_eq!(r.riga(1).as_bytes(), b"I avvio");
        assert!(r.riga(2).is_empty());
    }

    #[test]
    fn anello_scarta_la_piu_vecchia() {
        let mut r = Registri::new();
        for _ in 0..VOCI {
            r.push(Livello::Info, b"x");
        }
        r.push(Livello::Avviso, b"nuova");
        assert_eq!(r.riga(0).as_bytes(), b"A nuova");
        assert_eq!(r.riga(VOCI - 1).as_bytes(), b"I x");
        assert_eq!(r.voce(VOCI), None);
    }

    #[test]
    fn conteggi() {
        let mut r = Registri::new();
        r.push(Livello::Errore, b"a");
        r.push(Livello::Info, b"b");
        r.push(Livello::Errore, b"c");
        let (a, b) = r.out();
        assert_eq!(a.as_bytes(), b"LOG 3");
        assert_eq!(b.as_bytes(), b"ERRORI 2");
    }
}
