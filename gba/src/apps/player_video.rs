//! Lettore video: solo linea temporale (fotogrammi, posizione, play e pausa).
//! Nessun decoder: il video richiede ARM64 o x86, sul GBA è **non supportato**.
//! Il decoder sarà un programma separato che fornisce i fotogrammi a questa logica.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stato {
    Vuoto,
    Pausa,
    Suona,
    Fine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Video {
    fotogrammi: u32,
    fps: u32,
    frame: u32,
    /// Millisecondi totali riprodotti.
    tempo_ms: u64,
    stato: Stato,
}

impl Video {
    pub const fn new() -> Self {
        Video { fotogrammi: 0, fps: 1, frame: 0, tempo_ms: 0, stato: Stato::Vuoto }
    }

    /// Carica un video di `fotogrammi` a `fps` (0 non ammesso).
    pub fn carica(&mut self, fotogrammi: u32, fps: u32) -> bool {
        if fps == 0 || fotogrammi == 0 {
            return false;
        }
        *self = Video { fotogrammi, fps, frame: 0, tempo_ms: 0, stato: Stato::Pausa };
        true
    }

    pub fn play(&mut self) -> bool {
        match self.stato {
            Stato::Pausa => {
                self.stato = Stato::Suona;
                true
            }
            Stato::Fine => {
                self.frame = 0;
                self.tempo_ms = 0;
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

    /// Avanza di `ms` millisecondi. Restituisce il numero di fotogrammi da mostrare.
    pub fn avanza_ms(&mut self, ms: u32) -> u32 {
        if self.stato != Stato::Suona {
            return 0;
        }
        self.tempo_ms += ms as u64;
        let dovuti = (self.tempo_ms * self.fps as u64 / 1000) as u32;
        let resta = self.fotogrammi - self.frame;
        let letti = dovuti.saturating_sub(self.frame).min(resta);
        self.frame += letti;
        if self.frame == self.fotogrammi {
            self.stato = Stato::Fine;
        }
        letti
    }

    pub fn fotogramma(&self) -> u32 {
        self.frame
    }

    pub fn stato(&self) -> Stato {
        if self.fotogrammi == 0 { Stato::Vuoto } else { self.stato }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avanza_al_ritmo_dei_fps() {
        let mut v = Video::new();
        assert!(v.carica(30, 10));
        v.play();
        assert_eq!(v.avanza_ms(500), 5);
        assert_eq!(v.fotogramma(), 5);
        assert_eq!(v.avanza_ms(2500), 25);
        assert_eq!(v.stato(), Stato::Fine);
    }

    #[test]
    fn resto_millisecondi_non_si_perde() {
        let mut v = Video::new();
        v.carica(100, 3);
        v.play();
        let mut tot = 0;
        for _ in 0..1000 {
            tot += v.avanza_ms(1);
        }
        assert_eq!(tot, 3);
    }

    #[test]
    fn pausa_e_rifiuti() {
        let mut v = Video::new();
        assert!(!v.carica(0, 10));
        assert!(!v.carica(10, 0));
        assert!(!v.play());
        v.carica(10, 10);
        v.play();
        v.pausa();
        assert_eq!(v.avanza_ms(1000), 0);
    }
}
