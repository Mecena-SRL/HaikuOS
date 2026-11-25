//! Dizionario in sola lettura: tabella statica di voci in ordine alfabetico.
//! Ricerca binaria per parola esatta e per prefisso (primo indice e conteggio), navigazione su/giù.
//! Logica pura, solo `core`. Confronti senza distinzione di maiuscole (ASCII).
//! Nota: dizionario completo: non entra in 32 KB, da verificare. Qui c'è solo un campione.

#![allow(dead_code)]

use core::cmp::Ordering;

/// Numero di voci della tabella.
pub const N: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Voce {
    pub parola: &'static str,
    pub definizione: &'static str,
}

/// Tabella in ordine alfabetico (parole in minuscolo).
pub static VOCI: [Voce; N] = [
    Voce { parola: "acqua", definizione: "liquido trasparente, base della vita" },
    Voce { parola: "albero", definizione: "pianta alta con tronco e rami" },
    Voce { parola: "amico", definizione: "persona con cui si ha un legame di fiducia" },
    Voce { parola: "arancia", definizione: "frutto agrumato di colore arancio" },
    Voce { parola: "bicchiere", definizione: "recipiente di vetro per bere" },
    Voce { parola: "cane", definizione: "animale domestico che abbaia" },
    Voce { parola: "casa", definizione: "edificio in cui si abita" },
    Voce { parola: "cielo", definizione: "spazio sopra la terra" },
    Voce { parola: "cuore", definizione: "organo che pompa il sangue" },
    Voce { parola: "finestra", definizione: "apertura nel muro con vetro" },
    Voce { parola: "fiume", definizione: "corso d'acqua che scorre verso il mare" },
    Voce { parola: "gatto", definizione: "piccolo felino domestico" },
    Voce { parola: "libro", definizione: "insieme di fogli stampati e rilegati" },
    Voce { parola: "luce", definizione: "ciò che permette di vedere" },
    Voce { parola: "mare", definizione: "grande distesa di acqua salata" },
    Voce { parola: "montagna", definizione: "rilievo molto alto della terra" },
    Voce { parola: "notte", definizione: "periodo di buio tra sera e mattina" },
    Voce { parola: "pane", definizione: "alimento di farina cotto al forno" },
    Voce { parola: "sole", definizione: "stella intorno a cui gira la terra" },
    Voce { parola: "tavolo", definizione: "piano sostenuto da gambe" },
];

/// Confronto ASCII senza distinzione di maiuscole.
fn cmp_ci(a: &[u8], b: &[u8]) -> Ordering {
    a.iter()
        .map(|c| c.to_ascii_lowercase())
        .cmp(b.iter().map(|c| c.to_ascii_lowercase()))
}

/// Primo indice la cui parola non è minore di `p`; `N` se nessuna.
fn primo_non_minore(p: &[u8]) -> usize {
    let (mut lo, mut hi) = (0, N);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if cmp_ci(VOCI[mid].parola.as_bytes(), p) == Ordering::Less {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Indice della parola esatta (ricerca binaria).
pub fn cerca(parola: &str) -> Option<usize> {
    let p = parola.as_bytes();
    let i = primo_non_minore(p);
    if i < N && cmp_ci(VOCI[i].parola.as_bytes(), p) == Ordering::Equal {
        Some(i)
    } else {
        None
    }
}

/// (primo indice, conteggio) delle parole che iniziano per `p`. Conteggio 0 se nessuna.
pub fn cerca_prefisso(p: &str) -> (usize, usize) {
    let pb = p.as_bytes();
    let start = primo_non_minore(pb);
    let mut n = 0;
    while start + n < N {
        let w = VOCI[start + n].parola.as_bytes();
        if w.len() < pb.len() || cmp_ci(&w[..pb.len()], pb) != Ordering::Equal {
            break;
        }
        n += 1;
    }
    (start, n)
}

/// Navigazione su un intervallo di voci: tutte, oppure quelle di un prefisso.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sfoglia {
    lo: usize,
    hi: usize,
    cur: usize,
}

impl Sfoglia {
    pub const fn tutte() -> Self {
        Sfoglia { lo: 0, hi: N, cur: 0 }
    }

    /// Limita la navigazione alle parole che iniziano per `p`. Vuota se non ce ne sono.
    pub fn filtra(&mut self, p: &str) {
        let (s, n) = cerca_prefisso(p);
        self.lo = s;
        self.hi = s + n;
        self.cur = s;
    }

    /// Numero di voci nell'intervallo attivo.
    pub fn conteggio(&self) -> usize {
        self.hi - self.lo
    }

    /// Posizione della selezione dentro l'intervallo attivo.
    pub fn posizione(&self) -> usize {
        self.cur - self.lo
    }

    /// Voce selezionata; `None` se l'intervallo è vuoto.
    pub fn voce(&self) -> Option<&'static Voce> {
        if self.cur < self.hi { Some(&VOCI[self.cur]) } else { None }
    }

    /// Voce precedente, senza giro. `false` se è la prima.
    pub fn su(&mut self) -> bool {
        if self.cur > self.lo {
            self.cur -= 1;
            true
        } else {
            false
        }
    }

    /// Voce successiva, senza giro. `false` se è l'ultima.
    pub fn giu(&mut self) -> bool {
        if self.cur + 1 < self.hi {
            self.cur += 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabella_ordinata_senza_doppi() {
        for i in 1..N {
            let a = VOCI[i - 1].parola.as_bytes();
            let b = VOCI[i].parola.as_bytes();
            assert_eq!(cmp_ci(a, b), Ordering::Less, "fuori ordine: {} / {}", VOCI[i - 1].parola, VOCI[i].parola);
        }
    }

    #[test]
    fn ricerca_esatta() {
        assert_eq!(cerca("acqua"), Some(0));
        assert_eq!(cerca("tavolo"), Some(N - 1));
        assert_eq!(cerca("casa"), Some(6));
        assert_eq!(cerca("CASA"), Some(6));
        assert_eq!(cerca("cas"), None);
        assert_eq!(cerca("zebra"), None);
        assert_eq!(cerca(""), None);
        for (i, v) in VOCI.iter().enumerate() {
            assert_eq!(cerca(v.parola), Some(i));
        }
    }

    #[test]
    fn ricerca_per_prefisso() {
        assert_eq!(cerca_prefisso("ca"), (5, 2)); // cane, casa
        assert_eq!(cerca_prefisso("a"), (0, 4)); // acqua, albero, amico, arancia
        assert_eq!(cerca_prefisso("MON"), (15, 1));
        assert_eq!(cerca_prefisso(""), (0, N));
        assert_eq!(cerca_prefisso("zz").1, 0);
        assert_eq!(cerca_prefisso("fi"), (9, 2)); // finestra, fiume
        assert_eq!(cerca_prefisso("casaa").1, 0);
    }

    #[test]
    fn sfoglia_tutte_con_limiti() {
        let mut s = Sfoglia::tutte();
        assert_eq!(s.conteggio(), N);
        assert!(!s.su());
        assert_eq!(s.voce().unwrap().parola, "acqua");
        for _ in 0..N - 1 {
            assert!(s.giu());
        }
        assert!(!s.giu());
        assert_eq!(s.voce().unwrap().parola, "tavolo");
        assert_eq!(s.posizione(), N - 1);
    }

    #[test]
    fn sfoglia_filtrata() {
        let mut s = Sfoglia::tutte();
        s.filtra("ca");
        assert_eq!(s.conteggio(), 2);
        assert_eq!(s.voce().unwrap().parola, "cane");
        assert!(s.giu());
        assert_eq!(s.voce().unwrap().parola, "casa");
        assert!(!s.giu());
        assert!(s.su());
        assert!(!s.su());
        assert_eq!(s.posizione(), 0);

        s.filtra("zz");
        assert_eq!(s.conteggio(), 0);
        assert!(s.voce().is_none());
        assert!(!s.giu());
        assert!(!s.su());
    }
}
