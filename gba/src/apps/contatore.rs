//! Contatore a cifre (Tally): su, giù, azzera. Da 0 a 9999. Logica pura.

pub const MAX: u16 = 9999;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Contatore(u16);

impl Contatore {
    pub const fn new() -> Self {
        Contatore(0)
    }

    pub fn su(&mut self) {
        if self.0 < MAX {
            self.0 += 1;
        }
    }

    pub fn giu(&mut self) {
        self.0 = self.0.saturating_sub(1);
    }

    pub fn azzera(&mut self) {
        self.0 = 0;
    }

    pub fn valore(&self) -> u16 {
        self.0
    }

    /// Quattro cifre ASCII con zeri a sinistra, es. "0042".
    pub fn testo(&self) -> [u8; 4] {
        let v = self.0;
        [b'0' + (v / 1000 % 10) as u8, b'0' + (v / 100 % 10) as u8, b'0' + (v / 10 % 10) as u8, b'0' + (v % 10) as u8]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn conta_e_non_scende_sotto_zero() {
        let mut c = Contatore::new();
        c.giu();
        assert_eq!(c.valore(), 0);
        c.su();
        c.su();
        assert_eq!(c.testo(), *b"0002");
        c.azzera();
        assert_eq!(c.valore(), 0);
    }

    #[test]
    fn si_ferma_al_massimo() {
        let mut c = Contatore(MAX);
        c.su();
        assert_eq!(c.testo(), *b"9999");
    }
}
