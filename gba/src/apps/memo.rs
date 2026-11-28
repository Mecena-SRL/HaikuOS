//! Memo vocali: fino a 8 registrazioni (nome ≤12 byte, durata in campioni).
//! Macchina a stati: registra, ferma, riproduci, pausa; selezione ed eliminazione.
//! Nessun audio reale. Nota: registrazione da verificare: il GBA ha solo un microfono
//! di fatto assente, da verificare.

pub const MAX_REG: usize = 8;
pub const NOME_MAX: usize = 12;

/// Registrazione: nome fino a 12 byte e durata in campioni.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Registrazione {
    nome: [u8; NOME_MAX],
    len: u8,
    pub durata: u32,
}

const REG_VUOTA: Registrazione = Registrazione { nome: [0; NOME_MAX], len: 0, durata: 0 };

impl Registrazione {
    /// Crea una registrazione; `None` se il nome supera 12 byte.
    pub fn new(nome: &[u8], durata: u32) -> Option<Self> {
        if nome.len() > NOME_MAX {
            return None;
        }
        let mut n = [0u8; NOME_MAX];
        n[..nome.len()].copy_from_slice(nome);
        Some(Registrazione { nome: n, len: nome.len() as u8, durata })
    }

    pub fn nome(&self) -> &[u8] {
        &self.nome[..self.len as usize]
    }
}

/// Nome predefinito "Memo NN" per la registrazione di indice `i`.
fn nome_predefinito(i: usize) -> [u8; 7] {
    let n = i + 1;
    [b'M', b'e', b'm', b'o', b' ', b'0' + (n / 10) as u8, b'0' + (n % 10) as u8]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stato {
    Inattivo,
    Registrando,
    Riproduce,
    InPausa,
}

/// Elenco di memo vocali con stato di registrazione e riproduzione.
/// Selezione ed eliminazione sono possibili solo da fermo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Memo {
    reg: [Registrazione; MAX_REG],
    len: usize,
    sel: usize,
    stato: Stato,
    /// Posizione di riproduzione in campioni.
    pos: u32,
}

impl Memo {
    pub const fn new() -> Self {
        Memo {
            reg: [REG_VUOTA; MAX_REG],
            len: 0,
            sel: 0,
            stato: Stato::Inattivo,
            pos: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn registrazione(&self, i: usize) -> Option<&Registrazione> {
        if i < self.len {
            Some(&self.reg[i])
        } else {
            None
        }
    }

    /// Indice della registrazione selezionata, `None` se l'elenco è vuoto.
    pub fn selezione(&self) -> Option<usize> {
        if self.len > 0 {
            Some(self.sel)
        } else {
            None
        }
    }

    pub fn stato(&self) -> Stato {
        self.stato
    }

    pub fn posizione(&self) -> u32 {
        self.pos
    }

    /// Sposta la selezione di `delta`, fermandosi ai bordi (senza giro).
    /// `false` se non si può spostare (non da fermo o elenco vuoto).
    pub fn sposta(&mut self, delta: i32) -> bool {
        if self.stato != Stato::Inattivo || self.len == 0 {
            return false;
        }
        let nuovo = (self.sel as i32 + delta).clamp(0, self.len as i32 - 1);
        self.sel = nuovo as usize;
        true
    }

    /// Inizia una nuova registrazione in coda e la seleziona.
    /// `None` se l'elenco è pieno o non si è da fermo.
    pub fn registra(&mut self) -> Option<usize> {
        if self.stato != Stato::Inattivo || self.len >= MAX_REG {
            return None;
        }
        self.reg[self.len] = Registrazione::new(&nome_predefinito(self.len), 0)?;
        self.sel = self.len;
        self.len += 1;
        self.stato = Stato::Registrando;
        self.pos = 0;
        Some(self.sel)
    }

    /// Aggiunge `campioni` alla registrazione in corso. `false` se non si sta registrando.
    pub fn campiona(&mut self, campioni: u32) -> bool {
        if self.stato != Stato::Registrando {
            return false;
        }
        let r = &mut self.reg[self.sel];
        r.durata = r.durata.saturating_add(campioni);
        true
    }

    /// Ferma registrazione o riproduzione e torna inattivo. La durata resta salvata.
    pub fn ferma(&mut self) {
        self.stato = Stato::Inattivo;
        self.pos = 0;
    }

    /// Riproduce la registrazione selezionata dall'inizio, o riprende dalla pausa.
    pub fn riproduci(&mut self) -> bool {
        if self.len == 0 {
            return false;
        }
        match self.stato {
            Stato::Inattivo => {
                self.stato = Stato::Riproduce;
                self.pos = 0;
                true
            }
            Stato::InPausa => {
                self.stato = Stato::Riproduce;
                true
            }
            _ => false,
        }
    }

    pub fn pausa(&mut self) -> bool {
        if self.stato == Stato::Riproduce {
            self.stato = Stato::InPausa;
            true
        } else {
            false
        }
    }

    /// Avanza la riproduzione di `campioni`. Restituisce `true` quando la
    /// registrazione finisce: lo stato torna inattivo.
    pub fn avanza(&mut self, campioni: u32) -> bool {
        if self.stato != Stato::Riproduce {
            return false;
        }
        self.pos = self.pos.saturating_add(campioni);
        if self.pos >= self.reg[self.sel].durata {
            self.stato = Stato::Inattivo;
            self.pos = 0;
            true
        } else {
            false
        }
    }

    /// Elimina la registrazione selezionata (solo da fermo). Le successive scalano indietro.
    pub fn elimina(&mut self) -> bool {
        if self.stato != Stato::Inattivo || self.len == 0 {
            return false;
        }
        for i in self.sel..self.len - 1 {
            self.reg[i] = self.reg[i + 1];
        }
        self.len -= 1;
        self.reg[self.len] = REG_VUOTA;
        self.sel = self.sel.min(self.len.saturating_sub(1));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn con_registrazioni(n: usize) -> Memo {
        let mut m = Memo::new();
        for _ in 0..n {
            m.registra().unwrap();
            m.campiona(10);
            m.ferma();
        }
        m
    }

    #[test]
    fn nome_fino_a_12_byte() {
        assert!(Registrazione::new(b"123456789012", 0).is_some());
        assert!(Registrazione::new(b"1234567890123", 0).is_none());
    }

    #[test]
    fn registra_crea_e_seleziona() {
        let mut m = Memo::new();
        assert_eq!(m.selezione(), None);
        assert_eq!(m.registra(), Some(0));
        assert_eq!(m.stato(), Stato::Registrando);
        assert_eq!(m.selezione(), Some(0));
        assert_eq!(m.registrazione(0).unwrap().nome(), &b"Memo 01"[..]);
    }

    #[test]
    fn non_si_registra_due_volte_insieme() {
        let mut m = Memo::new();
        m.registra().unwrap();
        assert_eq!(m.registra(), None);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn massimo_otto_registrazioni() {
        let mut m = con_registrazioni(MAX_REG);
        assert_eq!(m.len(), MAX_REG);
        assert_eq!(m.registra(), None);
        assert_eq!(m.registrazione(7).unwrap().nome(), &b"Memo 08"[..]);
    }

    #[test]
    fn campiona_accumula_solo_mentre_registra() {
        let mut m = Memo::new();
        assert!(!m.campiona(100));
        m.registra().unwrap();
        assert!(m.campiona(100));
        assert!(m.campiona(50));
        assert_eq!(m.registrazione(0).unwrap().durata, 150);
        m.ferma();
        assert_eq!(m.stato(), Stato::Inattivo);
        assert_eq!(m.registrazione(0).unwrap().durata, 150);
        assert!(!m.campiona(1));
    }

    #[test]
    fn riproduci_pausa_e_fine() {
        let mut m = con_registrazioni(1);
        assert!(m.riproduci());
        assert_eq!(m.stato(), Stato::Riproduce);
        assert!(!m.avanza(4));
        assert!(m.pausa());
        assert!(!m.avanza(100));
        assert_eq!(m.posizione(), 4);
        assert!(m.riproduci());
        assert!(m.avanza(6));
        assert_eq!(m.stato(), Stato::Inattivo);
        assert_eq!(m.posizione(), 0);
    }

    #[test]
    fn riproduci_senza_registrazioni_non_parte() {
        let mut m = Memo::new();
        assert!(!m.riproduci());
        assert_eq!(m.stato(), Stato::Inattivo);
    }

    #[test]
    fn non_si_riproduce_mentre_si_registra() {
        let mut m = Memo::new();
        m.registra().unwrap();
        assert!(!m.riproduci());
        assert!(!m.pausa());
        assert_eq!(m.stato(), Stato::Registrando);
    }

    #[test]
    fn sposta_si_ferma_ai_bordi() {
        let mut m = con_registrazioni(3);
        assert!(m.sposta(10));
        assert_eq!(m.selezione(), Some(2));
        assert!(m.sposta(-10));
        assert_eq!(m.selezione(), Some(0));
    }

    #[test]
    fn sposta_non_mentre_riproduce() {
        let mut m = con_registrazioni(2);
        m.sposta(-1);
        m.riproduci();
        assert!(!m.sposta(1));
        assert_eq!(m.selezione(), Some(0));
    }

    #[test]
    fn elimina_scala_le_successive() {
        let mut m = con_registrazioni(3);
        m.sposta(-1);
        assert!(m.elimina());
        assert_eq!(m.len(), 2);
        assert_eq!(m.registrazione(1).unwrap().nome(), &b"Memo 03"[..]);
        assert_eq!(m.selezione(), Some(1));
    }

    #[test]
    fn elimina_ultima_sposta_la_selezione() {
        let mut m = con_registrazioni(2);
        m.sposta(1);
        assert!(m.elimina());
        assert_eq!(m.selezione(), Some(0));
        assert!(m.elimina());
        assert_eq!(m.selezione(), None);
        assert!(!m.elimina());
    }

    #[test]
    fn elimina_non_mentre_registra_o_riproduce() {
        let mut m = con_registrazioni(1);
        m.riproduci();
        assert!(!m.elimina());
        m.ferma();
        m.registra().unwrap();
        assert!(!m.elimina());
        assert_eq!(m.len(), 2);
    }
}
