//! Lettore audio: riproduce PCM i16 mono senza compressione (niente mp3/ogg).
//! Solo stato e posizione, nessun suono: l'uscita audio arriva dal microkernel audio.
//! Audio su GBA: da verificare.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stato {
    Vuoto,
    Pausa,
    Suona,
    Fine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lettore {
    totale: u32,
    freq: u32,
    pos: u32,
    stato: Stato,
}

impl Lettore {
    pub const fn new() -> Self {
        Lettore { totale: 0, freq: 1, pos: 0, stato: Stato::Vuoto }
    }

    /// Carica un brano di `campioni` a `freq` Hz. `freq` zero non è ammessa.
    pub fn carica(&mut self, campioni: u32, freq: u32) -> bool {
        if freq == 0 {
            return false;
        }
        *self = Lettore { totale: campioni, freq, pos: 0, stato: Stato::Pausa };
        campioni > 0
    }

    /// Avvia o riprende. Su brano finito riparte dall'inizio.
    pub fn play(&mut self) -> bool {
        match self.stato {
            Stato::Pausa => {
                self.stato = Stato::Suona;
                true
            }
            Stato::Fine => {
                self.pos = 0;
                self.stato = Stato::Suona;
                true
            }
            _ => false,
        }
    }

    pub fn pausa(&mut self) {
        if self.stato == Stato::Suona {
            self.stato = Stato::Pausa;
        }
    }

    /// Torna all'inizio in pausa.
    pub fn stop(&mut self) {
        if self.stato != Stato::Vuoto {
            self.pos = 0;
            self.stato = Stato::Pausa;
        }
    }

    /// Avanza di `campioni` e restituisce quanti ne sono stati letti davvero.
    pub fn avanza(&mut self, campioni: u32) -> u32 {
        if self.stato != Stato::Suona {
            return 0;
        }
        let resta = self.totale - self.pos;
        let letti = campioni.min(resta);
        self.pos += letti;
        if self.pos == self.totale {
            self.stato = Stato::Fine;
        }
        letti
    }

    /// Secondi trascorsi (arrotondati per difetto).
    pub fn secondi(&self) -> u32 {
        self.pos / self.freq
    }

    pub fn stato(&self) -> Stato {
        if self.totale == 0 { Stato::Vuoto } else { self.stato }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carica_e_riproduce_fino_alla_fine() {
        let mut p = Lettore::new();
        assert!(p.carica(100, 10));
        assert!(p.play());
        assert_eq!(p.avanza(60), 60);
        assert_eq!(p.secondi(), 6);
        assert_eq!(p.avanza(60), 40);
        assert_eq!(p.stato(), Stato::Fine);
        assert_eq!(p.avanza(1), 0);
    }

    #[test]
    fn pausa_non_avanza() {
        let mut p = Lettore::new();
        p.carica(100, 10);
        p.play();
        p.avanza(10);
        p.pausa();
        assert_eq!(p.avanza(50), 0);
        assert!(p.play());
        assert_eq!(p.secondi(), 1);
    }

    #[test]
    fn stop_torna_all_inizio_e_fine_riparte() {
        let mut p = Lettore::new();
        p.carica(20, 10);
        p.play();
        p.avanza(20);
        assert!(p.play());
        assert_eq!(p.stato(), Stato::Suona);
        p.stop();
        assert_eq!((p.secondi(), p.stato()), (0, Stato::Pausa));
    }

    #[test]
    fn niente_brano_o_frequenza_zero() {
        let mut p = Lettore::new();
        assert!(!p.play());
        assert!(!p.carica(10, 0));
        assert!(!p.carica(0, 10));
        assert_eq!(p.stato(), Stato::Vuoto);
    }
}
