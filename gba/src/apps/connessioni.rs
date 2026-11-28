//! Connessioni: stato della rete e indirizzo IP. Logica pura.
//! Il net stack non è ancora collegato: lo stato iniziale è `Offline` (**da verificare** su hardware).

use super::sysutil::{Line, Out};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stato {
    Spenta,
    Offline,
    Cerca,
    Connessa,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rete {
    stato: Stato,
    ip: [u8; 4],
}

impl Rete {
    pub const fn new() -> Self {
        Rete { stato: Stato::Offline, ip: [0; 4] }
    }

    pub fn stato(&self) -> Stato {
        self.stato
    }

    pub fn imposta(&mut self, stato: Stato, ip: [u8; 4]) {
        self.stato = stato;
        self.ip = ip;
    }

    /// Riga 1: stato. Riga 2 (solo se connessa): ultimi due ottetti, `IP *.x.y`
    /// (l'indirizzo intero non entra in `LINE` byte).
    pub fn out(&self) -> Out {
        let s: &[u8] = match self.stato {
            Stato::Spenta => b"SPENTA",
            Stato::Offline => b"OFFLINE",
            Stato::Cerca => b"CERCA...",
            Stato::Connessa => b"CONNESSA",
        };
        let riga1 = Line::new().push(b"RETE ").push(s);
        let riga2 = if self.stato == Stato::Connessa {
            Line::new().push(b"IP *.").num(self.ip[2] as usize).push(b".").num(self.ip[3] as usize)
        } else {
            Line::new()
        };
        (riga1, riga2)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn offline_di_default() {
        let r = Rete::new();
        assert_eq!(r.stato(), Stato::Offline);
        assert_eq!(r.out().0.as_bytes(), b"RETE OFFLINE");
        assert!(r.out().1.is_empty());
    }

    #[test]
    fn connessa_mostra_ultimi_ottetti() {
        let mut r = Rete::new();
        r.imposta(Stato::Connessa, [10, 0, 2, 15]);
        assert_eq!(r.out().0.as_bytes(), b"RETE CONNESSA");
        assert_eq!(r.out().1.as_bytes(), b"IP *.2.15");
    }
}
