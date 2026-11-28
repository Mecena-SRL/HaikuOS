//! Visita guidata: passi in sequenza. `avanti` ritorna falso alla fine del tour.

pub const PASSI: [&[&str]; 4] = [
    &["BENVENUTO IN", "HAIKUOS."],
    &["LE APP STANNO", "NEL MENU.", "SCEGLINE UNA", "CON SU E GIU."],
    &["OGNI APP USA", "SOLO LA MEMORIA", "CHE LE SPETTA."],
    &["FINE DEL TOUR.", "RIFALLO DA", "AIUTO."],
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tour {
    passo: u8,
}

impl Tour {
    pub const fn new() -> Self {
        Tour { passo: 0 }
    }

    pub fn passo(&self) -> usize {
        self.passo as usize
    }

    pub fn righe(&self) -> &'static [&'static str] {
        PASSI[self.passo()]
    }

    pub fn finito(&self) -> bool {
        self.passo() + 1 == PASSI.len()
    }

    /// Passa al passo successivo. Ritorna falso se il tour è già finito.
    pub fn avanti(&mut self) -> bool {
        if self.finito() {
            return false;
        }
        self.passo += 1;
        true
    }

    pub fn indietro(&mut self) {
        self.passo = self.passo.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avanza_fino_alla_fine() {
        let mut t = Tour::new();
        let mut n = 0;
        while t.avanti() {
            n += 1;
        }
        assert_eq!(n, PASSI.len() - 1);
        assert!(t.finito());
        assert!(!t.avanti());
    }
}
