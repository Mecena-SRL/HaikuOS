//! Messaggi: fino a 4 conversazioni, ognuna con fino a 8 messaggi di al massimo 32 byte.
//! Logica pura: niente disegno, niente heap, solo `core`.
//! Invio reale: nessuna rete sul GBA, da verificare. `invia` aggiunge solo in locale.
//! Ricezione simulata: `ricevi` è una funzione esplicita, chi la chiama sceglie il testo.

pub const MAX_CONVERSAZIONI: usize = 4;
pub const MAX_MESSAGGI: usize = 8;
pub const TESTO_MAX: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mittente {
    Io,
    Altro,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Messaggio {
    testo: [u8; TESTO_MAX],
    len: u8,
    pub mittente: Mittente,
}

impl Messaggio {
    const VUOTO: Messaggio = Messaggio { testo: [0; TESTO_MAX], len: 0, mittente: Mittente::Altro };

    pub fn testo(&self) -> &[u8] {
        &self.testo[..self.len as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Conversazione {
    messaggi: [Messaggio; MAX_MESSAGGI],
    len: usize,
}

impl Conversazione {
    const VUOTA: Conversazione = Conversazione { messaggi: [Messaggio::VUOTO; MAX_MESSAGGI], len: 0 };
}

pub struct Messaggi {
    conv: [Conversazione; MAX_CONVERSAZIONI],
    n: usize,
    sel: usize,
}

impl Messaggi {
    pub const fn new() -> Self {
        Messaggi { conv: [Conversazione::VUOTA; MAX_CONVERSAZIONI], n: 0, sel: 0 }
    }

    pub fn conversazioni(&self) -> usize {
        self.n
    }

    /// Indice della conversazione selezionata (vale 0 se non ce ne sono).
    pub fn selezionata(&self) -> usize {
        self.sel
    }

    /// Crea una conversazione vuota e la seleziona. `false` se sono piene.
    pub fn nuova(&mut self) -> bool {
        if self.n == MAX_CONVERSAZIONI {
            return false;
        }
        self.conv[self.n] = Conversazione::VUOTA;
        self.sel = self.n;
        self.n += 1;
        true
    }

    /// Seleziona la conversazione `i`. `false` se non esiste.
    pub fn seleziona(&mut self, i: usize) -> bool {
        if i >= self.n {
            return false;
        }
        self.sel = i;
        true
    }

    /// Messaggi della conversazione `i`, in ordine di arrivo (vuoti se non esiste).
    pub fn messaggi(&self, i: usize) -> &[Messaggio] {
        if i >= self.n {
            return &[];
        }
        &self.conv[i].messaggi[..self.conv[i].len]
    }

    /// Invia un messaggio dalla conversazione selezionata (mittente `Io`).
    /// `false` se non ci sono conversazioni, il testo è vuoto o più lungo di 32 byte, o la conversazione è piena.
    pub fn invia(&mut self, testo: &[u8]) -> bool {
        self.aggiungi(self.sel, Mittente::Io, testo)
    }

    /// Simulazione di un messaggio in arrivo nella conversazione `i` (mittente `Altro`).
    pub fn ricevi(&mut self, i: usize, testo: &[u8]) -> bool {
        self.aggiungi(i, Mittente::Altro, testo)
    }

    fn aggiungi(&mut self, i: usize, mittente: Mittente, testo: &[u8]) -> bool {
        if i >= self.n || testo.is_empty() || testo.len() > TESTO_MAX {
            return false;
        }
        let c = &mut self.conv[i];
        if c.len == MAX_MESSAGGI {
            return false;
        }
        let mut corpo = [0u8; TESTO_MAX];
        corpo[..testo.len()].copy_from_slice(testo);
        c.messaggi[c.len] = Messaggio { testo: corpo, len: testo.len() as u8, mittente };
        c.len += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nuova_e_selezione() {
        let mut m = Messaggi::new();
        assert_eq!(m.conversazioni(), 0);
        assert!(!m.seleziona(0));
        assert!(m.nuova());
        assert!(m.nuova());
        assert_eq!(m.conversazioni(), 2);
        assert_eq!(m.selezionata(), 1);
        assert!(m.seleziona(0));
        assert_eq!(m.selezionata(), 0);
        assert!(!m.seleziona(2));
    }

    #[test]
    fn al_massimo_quattro_conversazioni() {
        let mut m = Messaggi::new();
        for _ in 0..MAX_CONVERSAZIONI {
            assert!(m.nuova());
        }
        assert!(!m.nuova());
        assert_eq!(m.conversazioni(), MAX_CONVERSAZIONI);
    }

    #[test]
    fn invia_senza_conversazioni_fallisce() {
        let mut m = Messaggi::new();
        assert!(!m.invia(b"ciao"));
    }

    #[test]
    fn invia_aggiunge_messaggio_io() {
        let mut m = Messaggi::new();
        m.nuova();
        assert!(m.invia(b"ciao"));
        let msgs = m.messaggi(0);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].testo(), b"ciao");
        assert_eq!(msgs[0].mittente, Mittente::Io);
    }

    #[test]
    fn invia_rifiuta_testo_vuoto_o_troppo_lungo() {
        let mut m = Messaggi::new();
        m.nuova();
        assert!(!m.invia(b""));
        assert!(!m.invia(&[b'a'; TESTO_MAX + 1]));
        assert!(m.invia(&[b'a'; TESTO_MAX]));
        assert_eq!(m.messaggi(0)[0].testo().len(), TESTO_MAX);
    }

    #[test]
    fn al_massimo_otto_messaggi_per_conversazione() {
        let mut m = Messaggi::new();
        m.nuova();
        for _ in 0..MAX_MESSAGGI {
            assert!(m.invia(b"x"));
        }
        assert!(!m.invia(b"x"));
        assert_eq!(m.messaggi(0).len(), MAX_MESSAGGI);
    }

    #[test]
    fn ricevi_simulato_e_ordine() {
        let mut m = Messaggi::new();
        m.nuova();
        m.nuova(); // selezionata: la 1
        assert!(m.ricevi(1, b"ehi"));
        assert!(m.invia(b"ok")); // va nella selezionata (1)
        assert!(!m.ricevi(2, b"fuori"));
        assert_eq!(m.messaggi(0).len(), 0);
        let msgs = m.messaggi(1);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].mittente, Mittente::Altro);
        assert_eq!(msgs[0].testo(), b"ehi");
        assert_eq!(msgs[1].mittente, Mittente::Io);
        assert_eq!(msgs[1].testo(), b"ok");
    }

    #[test]
    fn messaggi_di_conversazione_inesistente_vuoti() {
        let m = Messaggi::new();
        assert!(m.messaggi(3).is_empty());
    }
}
