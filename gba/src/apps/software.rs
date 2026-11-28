//! Software: catalogo di app installabili. Stato in un bitmask (nessuna allocazione, nessuna rete).

pub const CATALOGO: [&str; 6] = ["CALC", "NOTES", "CLOCK", "CALENDAR", "MEMO", "VIEWER"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Software {
    sel: u8,
    installati: u8,
}

impl Software {
    pub const fn new() -> Self {
        Software { sel: 0, installati: 0 }
    }

    pub fn nome(&self, i: usize) -> &'static str {
        CATALOGO[i]
    }

    pub fn sel(&self) -> usize {
        self.sel as usize
    }

    pub fn su(&mut self) {
        self.sel = self.sel.saturating_sub(1);
    }

    pub fn giu(&mut self) {
        if (self.sel as usize) + 1 < CATALOGO.len() {
            self.sel += 1;
        }
    }

    /// Vero se il pacchetto `i` è installato.
    pub fn installato(&self, i: usize) -> bool {
        self.installati & (1 << i) != 0
    }

    /// Installa o disinstalla il pacchetto selezionato. Ritorna il nuovo stato.
    pub fn commuta(&mut self) -> bool {
        self.installati ^= 1 << self.sel;
        self.installato(self.sel())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commuta_installa_e_rimuove() {
        let mut s = Software::new();
        s.giu();
        assert!(s.commuta());
        assert!(s.installato(1));
        assert!(!s.commuta());
        assert!(!s.installato(1));
    }

    #[test]
    fn selezione_non_esce_dal_catalogo() {
        let mut s = Software::new();
        s.su();
        assert_eq!(s.sel(), 0);
        for _ in 0..20 {
            s.giu();
        }
        assert_eq!(s.sel(), CATALOGO.len() - 1);
    }
}
