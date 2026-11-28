//! Libreria foto: fino a 16 foto, ognuna con nome (≤12 byte), dimensioni e id di file nel FS (u8).
//! Logica pura: niente disegno, niente heap. Le pagine sono da 6 miniature.

pub const MAX: usize = 16;
pub const NOME_MAX: usize = 12;
pub const PER_PAGINA: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Foto {
    nome: [u8; NOME_MAX],
    nome_len: u8,
    pub w: u16,
    pub h: u16,
    pub file: u8,
}

impl Foto {
    const VUOTA: Foto = Foto { nome: [0; NOME_MAX], nome_len: 0, w: 0, h: 0, file: 0 };

    /// `None` se il nome è vuoto o più lungo di 12 byte, o se una dimensione è zero.
    pub fn new(nome: &[u8], w: u16, h: u16, file: u8) -> Option<Foto> {
        if nome.is_empty() || nome.len() > NOME_MAX || w == 0 || h == 0 {
            return None;
        }
        let mut buf = [0u8; NOME_MAX];
        buf[..nome.len()].copy_from_slice(nome);
        Some(Foto { nome: buf, nome_len: nome.len() as u8, w, h, file })
    }

    pub fn nome(&self) -> &[u8] {
        &self.nome[..self.nome_len as usize]
    }
}

pub struct Photos {
    items: [Foto; MAX],
    len: usize,
    cur: usize,
}

impl Photos {
    pub const fn new() -> Self {
        Photos { items: [Foto::VUOTA; MAX], len: 0, cur: 0 }
    }

    pub fn count(&self) -> usize {
        self.len
    }

    /// Indice della foto selezionata (0 se la libreria è vuota).
    pub fn selezionato(&self) -> usize {
        self.cur
    }

    pub fn get(&self, i: usize) -> Option<&Foto> {
        if i < self.len { Some(&self.items[i]) } else { None }
    }

    pub fn selezionata(&self) -> Option<&Foto> {
        self.get(self.cur)
    }

    /// Aggiunge una foto in coda e la seleziona. `false` se la libreria è piena,
    /// i dati non sono validi (vedi `Foto::new`) o l'id di file è già in uso.
    pub fn aggiungi(&mut self, nome: &[u8], w: u16, h: u16, file: u8) -> bool {
        if self.len == MAX || self.items[..self.len].iter().any(|f| f.file == file) {
            return false;
        }
        let Some(foto) = Foto::new(nome, w, h, file) else {
            return false;
        };
        self.items[self.len] = foto;
        self.len += 1;
        self.cur = self.len - 1;
        true
    }

    /// Rimuove la foto in `i` e la restituisce. `None` se `i` è fuori indice.
    /// La selezione resta sulla foto che segue (o sull'ultima, se era l'ultima).
    pub fn rimuovi(&mut self, i: usize) -> Option<Foto> {
        if i >= self.len {
            return None;
        }
        let tolta = self.items[i];
        for k in i..self.len - 1 {
            self.items[k] = self.items[k + 1];
        }
        self.len -= 1;
        self.items[self.len] = Foto::VUOTA;
        if i < self.cur {
            self.cur -= 1;
        }
        if self.cur >= self.len {
            self.cur = self.len.saturating_sub(1);
        }
        Some(tolta)
    }

    /// Foto precedente, con giro: dalla prima va all'ultima.
    pub fn su(&mut self) {
        if self.len == 0 {
            return;
        }
        self.cur = if self.cur == 0 { self.len - 1 } else { self.cur - 1 };
    }

    /// Foto successiva, con giro: dall'ultima va alla prima.
    pub fn giu(&mut self) {
        if self.len == 0 {
            return;
        }
        self.cur = if self.cur + 1 >= self.len { 0 } else { self.cur + 1 };
    }

    /// Numero di pagine (0 se la libreria è vuota).
    pub fn pagine(&self) -> usize {
        (self.len + PER_PAGINA - 1) / PER_PAGINA
    }

    /// Pagina della foto selezionata, da 0.
    pub fn pagina_corrente(&self) -> usize {
        if self.len == 0 { 0 } else { self.cur / PER_PAGINA }
    }

    /// Indici `[inizio, fine)` delle miniature della pagina corrente.
    pub fn intervallo_pagina(&self) -> (usize, usize) {
        let inizio = self.pagina_corrente() * PER_PAGINA;
        (inizio, (inizio + PER_PAGINA).min(self.len))
    }

    /// Pagina successiva, con giro; seleziona la sua prima foto.
    pub fn pagina_succ(&mut self) {
        let n = self.pagine();
        if n <= 1 {
            return;
        }
        self.cur = ((self.pagina_corrente() + 1) % n) * PER_PAGINA;
    }

    /// Pagina precedente, con giro; seleziona la sua prima foto.
    pub fn pagina_prec(&mut self) {
        let n = self.pagine();
        if n <= 1 {
            return;
        }
        let p = self.pagina_corrente();
        let prec = if p == 0 { n - 1 } else { p - 1 };
        self.cur = prec * PER_PAGINA;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn aggiungi_seleziona_la_nuova_foto() {
        let mut p = super::Photos::new();
        assert_eq!(p.count(), 0);
        assert!(p.selezionata().is_none());
        assert!(p.aggiungi(b"Gatto", 64, 48, 1));
        assert!(p.aggiungi(b"Cane", 32, 32, 2));
        assert_eq!(p.count(), 2);
        assert_eq!(p.selezionato(), 1);
        let f = p.selezionata().unwrap();
        assert_eq!(f.nome(), b"Cane");
        assert_eq!((f.w, f.h, f.file), (32, 32, 2));
    }

    #[test]
    fn aggiungi_rifiuta_dati_non_validi() {
        let mut p = super::Photos::new();
        assert!(!p.aggiungi(b"", 1, 1, 1)); // nome vuoto
        assert!(!p.aggiungi(b"Tredici bytes", 1, 1, 1)); // 13 byte
        assert!(p.aggiungi(b"DodiciByte12", 1, 1, 2)); // esattamente 12
        assert!(!p.aggiungi(b"Zero", 0, 10, 3)); // larghezza zero
        assert!(!p.aggiungi(b"Zero", 10, 0, 3)); // altezza zero
        assert_eq!(p.count(), 1);
    }

    #[test]
    fn aggiungi_rifiuta_id_file_duplicato() {
        let mut p = super::Photos::new();
        assert!(p.aggiungi(b"A", 1, 1, 7));
        assert!(!p.aggiungi(b"B", 1, 1, 7));
        assert_eq!(p.count(), 1);
    }

    #[test]
    fn libreria_piena_a_16() {
        let mut p = super::Photos::new();
        for i in 0..16u8 {
            assert!(p.aggiungi(b"foto", 8, 8, i));
        }
        assert_eq!(p.count(), 16);
        assert!(!p.aggiungi(b"extra", 8, 8, 99));
        assert_eq!(p.count(), 16);
    }

    #[test]
    fn rimuovi_sposta_e_aggiorna_selezione() {
        let mut p = super::Photos::new();
        for i in 0..4u8 {
            p.aggiungi(&[b'a' + i], 8, 8, i);
        }
        p.su(); // seleziona indice 2
        assert_eq!(p.selezionato(), 2);
        // rimuovo una foto prima della selezione: la selezione segue la stessa foto
        let tolta = p.rimuovi(0).unwrap();
        assert_eq!(tolta.file, 0);
        assert_eq!(p.count(), 3);
        assert_eq!(p.selezionato(), 1);
        assert_eq!(p.selezionata().unwrap().file, 2);
        // rimuovo l'ultima con selezione su di essa: resta sull'ultima rimasta
        p.giu();
        assert_eq!(p.selezionato(), 2);
        assert!(p.rimuovi(2).is_some());
        assert_eq!(p.selezionato(), 1);
        assert_eq!(p.count(), 2);
        assert!(p.rimuovi(5).is_none());
    }

    #[test]
    fn rimuovi_libreria_fino_a_vuota() {
        let mut p = super::Photos::new();
        p.aggiungi(b"x", 1, 1, 1);
        assert!(p.rimuovi(0).is_some());
        assert_eq!(p.count(), 0);
        assert_eq!(p.selezionato(), 0);
        assert!(p.selezionata().is_none());
        p.su();
        p.giu();
        assert_eq!(p.selezionato(), 0);
    }

    #[test]
    fn su_e_giu_fanno_il_giro() {
        let mut p = super::Photos::new();
        for i in 0..3u8 {
            p.aggiungi(b"f", 1, 1, i);
        }
        assert_eq!(p.selezionato(), 2);
        p.giu();
        assert_eq!(p.selezionato(), 0); // giro in avanti
        p.su();
        assert_eq!(p.selezionato(), 2); // giro indietro
        p.su();
        assert_eq!(p.selezionato(), 1);
    }

    #[test]
    fn pagine_da_sei_miniature() {
        let mut p = super::Photos::new();
        assert_eq!(p.pagine(), 0);
        for i in 0..16u8 {
            p.aggiungi(b"f", 1, 1, i);
        }
        assert_eq!(p.pagine(), 3); // 6 + 6 + 4
        assert_eq!(p.pagina_corrente(), 2); // selezionata l'ultima
        assert_eq!(p.intervallo_pagina(), (12, 16));
    }

    #[test]
    fn pagina_succ_e_prec_con_giro() {
        let mut p = super::Photos::new();
        for i in 0..13u8 {
            p.aggiungi(b"f", 1, 1, i);
        }
        assert_eq!(p.pagine(), 3);
        assert_eq!(p.pagina_corrente(), 2);
        p.pagina_succ(); // giro alla prima
        assert_eq!((p.pagina_corrente(), p.selezionato()), (0, 0));
        assert_eq!(p.intervallo_pagina(), (0, 6));
        p.pagina_prec(); // giro all'ultima
        assert_eq!((p.pagina_corrente(), p.selezionato()), (2, 12));
        assert_eq!(p.intervallo_pagina(), (12, 13));
        p.pagina_prec();
        assert_eq!(p.intervallo_pagina(), (6, 12));
    }

    #[test]
    fn pagina_con_una_sola_pagina_non_cambia() {
        let mut p = super::Photos::new();
        p.aggiungi(b"f", 1, 1, 1);
        p.pagina_succ();
        p.pagina_prec();
        assert_eq!(p.selezionato(), 0);
        assert_eq!(p.intervallo_pagina(), (0, 1));
    }

    #[test]
    fn nome_restituito_senza_residui() {
        let f = super::Foto::new(b"Mare", 2, 3, 9).unwrap();
        assert_eq!(f.nome(), b"Mare");
        assert_eq!(f.file, 9);
    }
}
