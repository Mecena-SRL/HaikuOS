//! Acquisizione documenti: macchina a stati dello scanner. Il GBA non ha scanner:
//! ogni scansione dà errore "scanner assente" finché non c'è un driver hardware.
//! Hardware necessario: scanner con alimentatore (ADF) o sensore d'immagine collegato via USB o porta dedicata.

pub const AVVISO: &[&str] = &["SERVE UNO", "SCANNER COLLEGATO", "DA VERIFICARE"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stato {
    Spento,
    Pronto,
    Acquisito,
    ScannerAssente,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evento {
    Accendi,
    Scansiona,
    Spegni,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scanner {
    stato: Stato,
}

impl Scanner {
    pub const fn new() -> Self {
        Scanner { stato: Stato::Spento }
    }

    pub fn stato(&self) -> Stato {
        self.stato
    }

    /// Applica un evento. Ritorna falso se l'evento non è valido nello stato attuale.
    pub fn evento(&mut self, e: Evento) -> bool {
        match (self.stato, e) {
            (_, Evento::Spegni) => self.stato = Stato::Spento,
            (Stato::Spento, Evento::Accendi) => self.stato = Stato::Pronto,
            (Stato::Pronto, Evento::Scansiona) => self.stato = Stato::ScannerAssente,
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn senza_hardware_la_scansione_fallisce() {
        let mut s = Scanner::new();
        assert!(!s.evento(Evento::Scansiona));
        s.evento(Evento::Accendi);
        assert!(s.evento(Evento::Scansiona));
        assert_eq!(s.stato(), Stato::ScannerAssente);
        assert_eq!(AVVISO.len(), 3);
    }
}
