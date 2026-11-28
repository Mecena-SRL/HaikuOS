//! Meteo: previsioni di 3 giorni con cache offline (ultimi dati ricevuti).
//! Nessuno stack di rete ancora: i dati arrivano da `aggiorna` quando la rete esiste.
//! Senza rete l'app mostra la cache con la sua età. Da fare: stack di rete.

pub const GIORNI: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cielo {
    Sereno,
    Nuvoloso,
    Pioggia,
    Neve,
    Temporale,
}

impl Cielo {
    pub fn etichetta(&self) -> &'static [u8] {
        match self {
            Cielo::Sereno => b"SERENO",
            Cielo::Nuvoloso => b"NUVOLOSO",
            Cielo::Pioggia => b"PIOGGIA",
            Cielo::Neve => b"NEVE",
            Cielo::Temporale => b"TEMPORALE",
        }
    }
}

/// Previsione di un giorno: temperature in °C (min <= max).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Giorno {
    pub min: i8,
    pub max: i8,
    pub cielo: Cielo,
}

impl Giorno {
    /// `None` se le temperature sono invertite.
    pub fn new(min: i8, max: i8, cielo: Cielo) -> Option<Self> {
        if min > max {
            return None;
        }
        Some(Giorno { min, max, cielo })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stato {
    /// Nessun dato mai ricevuto.
    SenzaDati,
    /// Dati vecchi di `ore` ore (cache offline).
    Cache { ore: u16 },
    /// Dati ricevuti in questo momento.
    Fresco,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Meteo {
    giorni: [Option<Giorno>; GIORNI],
    ore: u16,
    presenti: bool,
}

impl Meteo {
    pub const fn new() -> Self {
        Meteo { giorni: [None; GIORNI], ore: 0, presenti: false }
    }

    /// Sostituisce la cache con i dati ricevuti.
    pub fn aggiorna(&mut self, previsioni: [Giorno; GIORNI]) {
        self.giorni = previsioni.map(Some);
        self.ore = 0;
        self.presenti = true;
    }

    /// Fa invecchiare la cache (saturando a `u16::MAX` ore).
    pub fn trascorri_ore(&mut self, ore: u16) {
        if self.presenti {
            self.ore = self.ore.saturating_add(ore);
        }
    }

    pub fn giorno(&self, i: usize) -> Option<Giorno> {
        self.giorni.get(i).copied().flatten()
    }

    pub fn stato(&self) -> Stato {
        match (self.presenti, self.ore) {
            (false, _) => Stato::SenzaDati,
            (true, 0) => Stato::Fresco,
            (true, ore) => Stato::Cache { ore },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(min: i8, max: i8) -> Giorno {
        Giorno::new(min, max, Cielo::Sereno).unwrap()
    }

    #[test]
    fn giorno_rifiuta_temperature_invertite() {
        assert!(Giorno::new(10, 5, Cielo::Neve).is_none());
        assert!(Giorno::new(-3, -3, Cielo::Neve).is_some());
    }

    #[test]
    fn senza_dati_all_inizio() {
        let m = Meteo::new();
        assert_eq!(m.stato(), Stato::SenzaDati);
        assert_eq!(m.giorno(0), None);
    }

    #[test]
    fn cache_invecchia_e_si_rinnova() {
        let mut m = Meteo::new();
        m.aggiorna([g(1, 2), g(3, 4), g(5, 6)]);
        assert_eq!(m.stato(), Stato::Fresco);
        m.trascorri_ore(5);
        assert_eq!(m.stato(), Stato::Cache { ore: 5 });
        assert_eq!(m.giorno(2).map(|x| x.max), Some(6));
        m.aggiorna([g(0, 0); GIORNI]);
        assert_eq!(m.stato(), Stato::Fresco);
    }

    #[test]
    fn indice_fuori_range_non_panica() {
        let mut m = Meteo::new();
        m.aggiorna([g(0, 1); GIORNI]);
        assert_eq!(m.giorno(GIORNI), None);
        m.trascorri_ore(u16::MAX);
        m.trascorri_ore(1);
        assert_eq!(m.stato(), Stato::Cache { ore: u16::MAX });
    }
}
