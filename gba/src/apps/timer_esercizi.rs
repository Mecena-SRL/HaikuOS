//! Timer per esercizi: round di lavoro e riposo, in secondi. Logica pura, senza allocazioni.
//! Usa 60 frame al secondo come l'orologio (errore da misurare: da verificare).

pub const FPS: u32 = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fase {
    Lavoro,
    Riposo,
    Fine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timer {
    lavoro: u16,
    riposo: u16,
    round: u8,
    corrente: u8,
    fase: Fase,
    frames: u32,
    pausa: bool,
}

impl Timer {
    /// `lavoro` e `riposo` in secondi, `round` numero di round (0 = già finito).
    pub const fn new(lavoro: u16, riposo: u16, round: u8) -> Self {
        let fase = if round == 0 { Fase::Fine } else { Fase::Lavoro };
        Timer { lavoro, riposo, round, corrente: 1, fase, frames: lavoro as u32 * FPS, pausa: false }
    }

    pub fn fase(&self) -> Fase {
        self.fase
    }

    /// Round in corso, da 1 a `round`.
    pub fn round(&self) -> u8 {
        self.corrente
    }

    /// Secondi che restano nella fase, arrotondati per eccesso.
    pub fn secondi(&self) -> u32 {
        self.frames.div_ceil(FPS)
    }

    pub fn pausa(&mut self) {
        self.pausa = !self.pausa;
    }

    /// Avanza di `n` frame, passando da una fase all'altra quando finisce il tempo.
    pub fn tick(&mut self, mut n: u32) {
        if self.pausa {
            return;
        }
        while n > 0 && self.fase != Fase::Fine {
            if self.frames > n {
                self.frames -= n;
                return;
            }
            n -= self.frames;
            self.frames = 0;
            self.avanza();
        }
    }

    fn avanza(&mut self) {
        match self.fase {
            Fase::Lavoro if self.corrente == self.round => self.fase = Fase::Fine,
            Fase::Lavoro if self.riposo > 0 => self.passa(Fase::Riposo, self.riposo),
            Fase::Lavoro | Fase::Riposo => {
                self.corrente += 1;
                self.passa(Fase::Lavoro, self.lavoro);
            }
            Fase::Fine => {}
        }
    }

    fn passa(&mut self, fase: Fase, secondi: u16) {
        self.fase = fase;
        self.frames = secondi as u32 * FPS;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lavoro_riposo_poi_fine_all_ultimo_round() {
        let mut t = Timer::new(1, 1, 2);
        t.tick(60);
        assert_eq!(t.fase(), Fase::Riposo);
        t.tick(60);
        assert_eq!((t.fase(), t.round()), (Fase::Lavoro, 2));
        t.tick(60);
        assert_eq!(t.fase(), Fase::Fine);
    }

    #[test]
    fn senza_riposo_passa_subito_al_round_dopo() {
        let mut t = Timer::new(1, 0, 2);
        t.tick(60);
        assert_eq!((t.fase(), t.round()), (Fase::Lavoro, 2));
    }

    #[test]
    fn pausa_ferma_il_tempo_e_zero_round_e_finito() {
        let mut t = Timer::new(5, 5, 1);
        t.pausa();
        t.tick(1000);
        assert_eq!(t.secondi(), 5);
        assert_eq!(Timer::new(5, 5, 0).fase(), Fase::Fine);
    }
}
