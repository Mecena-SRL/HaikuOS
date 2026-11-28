//! Orologio per scacchi: due giocatori con tempo fisso ciascuno. Premere passa il turno all'altro.
//! Logica pura, senza allocazioni. 60 frame al secondo (da verificare sul GBA).

pub const FPS: u32 = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Orologio {
    frames: [u32; 2],
    /// Giocatore il cui tempo scorre (0 o 1), `None` se in pausa o scaduto.
    attivo: Option<u8>,
}

impl Orologio {
    /// `secondi` di tempo per ciascun giocatore. Parte fermo.
    pub const fn new(secondi: u32) -> Self {
        Orologio { frames: [secondi * FPS, secondi * FPS], attivo: None }
    }

    /// Premuto dal giocatore `p` (0 o 1): passa il turno all'altro. Da fermo avvia l'altro giocatore.
    pub fn premi(&mut self, p: u8) {
        if self.attivo.is_none_or(|a| a == p) {
            self.attivo = Some(1 - p);
        }
    }

    pub fn pausa(&mut self) {
        self.attivo = None;
    }

    pub fn tick(&mut self, n: u32) {
        if let Some(i) = self.attivo {
            let i = i as usize;
            self.frames[i] = self.frames[i].saturating_sub(n);
            if self.frames[i] == 0 {
                self.attivo = None; // bandiera: tempo scaduto
            }
        }
    }

    /// Secondi rimasti al giocatore `p`, per difetto.
    pub fn secondi(&self, p: u8) -> u32 {
        self.frames[p as usize] / FPS
    }

    pub fn scaduto(&self, p: u8) -> bool {
        self.frames[p as usize] == 0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn il_turno_passa_e_scorre_solo_un_giocatore() {
        let mut o = Orologio::new(10);
        o.premi(0); // avvia il giocatore 1
        o.tick(FPS * 3);
        assert_eq!((o.secondi(0), o.secondi(1)), (10, 7));
        o.premi(1); // il giocatore 1 passa al 0
        o.tick(FPS);
        assert_eq!((o.secondi(0), o.secondi(1)), (9, 7));
    }

    #[test]
    fn bandiera_ferma_il_tempo() {
        let mut o = Orologio::new(1);
        o.premi(0);
        o.tick(FPS);
        assert!(o.scaduto(1));
        o.tick(FPS);
        assert_eq!(o.secondi(0), 1);
    }

    #[test]
    fn pausa_ferma_tutto() {
        let mut o = Orologio::new(5);
        o.premi(0);
        o.pausa();
        o.tick(FPS * 10);
        assert_eq!(o.secondi(1), 5);
    }
}
