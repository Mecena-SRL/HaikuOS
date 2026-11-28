//! Calendario e promemoria: fino a 16 eventi, ognuno con titolo (≤16 byte), data e ora.
//! Logica pura: niente disegno, niente heap, solo `core`. Le note non sono previste.
//! Fuso orario: UTC, costante `FUSO_MINUTI` (da verificare, da decidere).

/// Scostamento dal UTC in minuti. Da decidere: per ora UTC. Da verificare.
pub const FUSO_MINUTI: i32 = 0;

pub const MAX_EVENTI: usize = 16;
pub const TITOLO_MAX: usize = 16;
pub const RIGHE_MESE: usize = 6;

/// Data (anno, mese 1..=12, giorno 1..=31). Ordinabile in ordine cronologico.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Data {
    pub anno: u16,
    pub mese: u8,
    pub giorno: u8,
}

impl Data {
    /// `None` se la data non esiste (es. 31 aprile, 29 febbraio di anno non bisestile).
    pub fn new(anno: u16, mese: u8, giorno: u8) -> Option<Self> {
        if mese == 0 || mese > 12 || giorno == 0 || giorno > giorni_nel_mese(anno, mese) {
            return None;
        }
        Some(Data { anno, mese, giorno })
    }
}

/// Ora del giorno (0..24, 0..60).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ora {
    pub h: u8,
    pub m: u8,
}

impl Ora {
    pub fn new(h: u8, m: u8) -> Option<Self> {
        if h < 24 && m < 60 { Some(Ora { h, m }) } else { None }
    }
}

/// Un evento. Il titolo è in un array fisso; `len` dice quanti byte sono usati.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Evento {
    titolo: [u8; TITOLO_MAX],
    len: u8,
    pub data: Data,
    pub ora: Ora,
}

impl Evento {
    const VUOTO: Evento = Evento {
        titolo: [0; TITOLO_MAX],
        len: 0,
        data: Data { anno: 0, mese: 1, giorno: 1 },
        ora: Ora { h: 0, m: 0 },
    };

    pub fn titolo(&self) -> &[u8] {
        &self.titolo[..self.len as usize]
    }
}

/// Elenco eventi, tenuto ordinato per (data, ora) a ogni inserimento.
pub struct Calendario {
    eventi: [Evento; MAX_EVENTI],
    len: usize,
}

impl Calendario {
    pub const fn new() -> Self {
        Calendario { eventi: [Evento::VUOTO; MAX_EVENTI], len: 0 }
    }

    pub fn count(&self) -> usize {
        self.len
    }

    /// Tutti gli eventi, in ordine cronologico.
    pub fn eventi(&self) -> &[Evento] {
        &self.eventi[..self.len]
    }

    pub fn evento(&self, i: usize) -> Option<&Evento> {
        if i < self.len { Some(&self.eventi[i]) } else { None }
    }

    /// Aggiunge un evento in ordine. `false` se è pieno, il titolo è vuoto o più lungo di 16 byte.
    /// A parità di data e ora, il nuovo evento va dopo quelli già presenti.
    pub fn aggiungi(&mut self, titolo: &[u8], data: Data, ora: Ora) -> bool {
        if self.len == MAX_EVENTI || titolo.is_empty() || titolo.len() > TITOLO_MAX {
            return false;
        }
        let mut nome = [0u8; TITOLO_MAX];
        nome[..titolo.len()].copy_from_slice(titolo);
        let nuovo = Evento { titolo: nome, len: titolo.len() as u8, data, ora };

        let chiave = (data, ora);
        let mut pos = self.len;
        while pos > 0 && (self.eventi[pos - 1].data, self.eventi[pos - 1].ora) > chiave {
            pos -= 1;
        }
        let mut i = self.len;
        while i > pos {
            self.eventi[i] = self.eventi[i - 1];
            i -= 1;
        }
        self.eventi[pos] = nuovo;
        self.len += 1;
        true
    }

    /// Rimuove l'evento `i`. `false` se non esiste.
    pub fn rimuovi(&mut self, i: usize) -> bool {
        if i >= self.len {
            return false;
        }
        for j in i..self.len - 1 {
            self.eventi[j] = self.eventi[j + 1];
        }
        self.len -= 1;
        self.eventi[self.len] = Evento::VUOTO;
        true
    }

    /// Eventi di un giorno, già ordinati per ora.
    pub fn del_giorno(&self, data: Data) -> impl Iterator<Item = &Evento> + '_ {
        self.eventi().iter().filter(move |e| e.data == data)
    }
}

/// Mese da mostrare (per la navigazione).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mese {
    pub anno: u16,
    pub mese: u8,
}

impl Mese {
    /// `None` se `mese` non è in 1..=12.
    pub fn new(anno: u16, mese: u8) -> Option<Self> {
        if mese == 0 || mese > 12 { None } else { Some(Mese { anno, mese }) }
    }

    /// Mese precedente, con giro d'anno (gennaio -> dicembre dell'anno prima).
    /// All'anno 0 resta a 0 (limite da verificare).
    pub fn precedente(self) -> Self {
        if self.mese == 1 {
            Mese { anno: self.anno.saturating_sub(1), mese: 12 }
        } else {
            Mese { anno: self.anno, mese: self.mese - 1 }
        }
    }

    /// Mese successivo, con giro d'anno (dicembre -> gennaio dell'anno dopo).
    pub fn successivo(self) -> Self {
        if self.mese == 12 {
            Mese { anno: self.anno.saturating_add(1), mese: 1 }
        } else {
            Mese { anno: self.anno, mese: self.mese + 1 }
        }
    }
}

pub fn bisestile(anno: u16) -> bool {
    anno % 4 == 0 && (anno % 100 != 0 || anno % 400 == 0)
}

/// Giorni del mese (28..=31). Vale 0 se `mese` non è in 1..=12.
pub fn giorni_nel_mese(anno: u16, mese: u8) -> u8 {
    match mese {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if bisestile(anno) { 29 } else { 28 }
        }
        _ => 0,
    }
}

/// Giorno della settimana, con lunedì = 0 e domenica = 6. Algoritmo di Sakamoto.
/// Vale 0 se `mese` non è in 1..=12.
pub fn giorno_settimana(anno: u16, mese: u8, giorno: u8) -> u8 {
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    if mese == 0 || mese > 12 {
        return 0;
    }
    // +400 anni non cambiano il giorno della settimana: evita i valori negativi.
    let y = anno as i32 + 400 - if mese < 3 { 1 } else { 0 };
    let s = (y + y / 4 - y / 100 + y / 400 + T[mese as usize - 1] + giorno as i32).rem_euclid(7);
    // Sakamoto dà 0 = domenica: lo riporto a lunedì = 0.
    ((s + 6) % 7) as u8
}

/// Griglia del mese: 6 righe da 7 colonne, prima colonna lunedì.
/// Le celle vuote valgono 0; le altre il numero del giorno.
pub fn griglia(anno: u16, mese: u8) -> [[u8; 7]; RIGHE_MESE] {
    let mut g = [[0u8; 7]; RIGHE_MESE];
    if mese == 0 || mese > 12 {
        return g;
    }
    let primo = giorno_settimana(anno, mese, 1) as usize;
    let n = giorni_nel_mese(anno, mese) as usize;
    for giorno in 1..=n {
        let idx = primo + giorno - 1;
        g[idx / 7][idx % 7] = giorno as u8;
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(anno: u16, mese: u8, giorno: u8) -> Data {
        Data::new(anno, mese, giorno).unwrap()
    }

    fn o(h: u8, m: u8) -> Ora {
        Ora::new(h, m).unwrap()
    }

    #[test]
    fn bisestili() {
        assert!(bisestile(2000));
        assert!(bisestile(2024));
        assert!(!bisestile(1900));
        assert!(!bisestile(2023));
        assert!(!bisestile(2100));
    }

    #[test]
    fn giorni_del_mese() {
        assert_eq!(giorni_nel_mese(2024, 2), 29);
        assert_eq!(giorni_nel_mese(2000, 2), 29);
        assert_eq!(giorni_nel_mese(1900, 2), 28);
        assert_eq!(giorni_nel_mese(2023, 2), 28);
        assert_eq!(giorni_nel_mese(2026, 4), 30);
        assert_eq!(giorni_nel_mese(2026, 12), 31);
        assert_eq!(giorni_nel_mese(2026, 13), 0);
    }

    #[test]
    fn data_non_valida() {
        assert_eq!(Data::new(2023, 2, 29), None);
        assert_eq!(Data::new(2026, 4, 31), None);
        assert_eq!(Data::new(2026, 0, 1), None);
        assert_eq!(Data::new(2026, 1, 0), None);
        assert!(Data::new(2024, 2, 29).is_some());
    }

    #[test]
    fn giorno_settimana_noti() {
        // 0 = lunedì, 5 = sabato, 6 = domenica.
        assert_eq!(giorno_settimana(2000, 1, 1), 5); // sabato
        assert_eq!(giorno_settimana(2024, 1, 1), 0); // lunedì
        assert_eq!(giorno_settimana(2024, 2, 29), 3); // giovedì
        assert_eq!(giorno_settimana(1900, 3, 1), 3); // giovedì
        assert_eq!(giorno_settimana(2001, 9, 9), 6); // domenica
    }

    #[test]
    fn griglia_febbraio_2024() {
        let g = griglia(2024, 2);
        // 1 febbraio 2024 è giovedì: colonna 3.
        assert_eq!(g[0], [0, 0, 0, 1, 2, 3, 4]);
        assert_eq!(g[4], [26, 27, 28, 29, 0, 0, 0]);
        assert_eq!(g[5], [0; 7]);
    }

    #[test]
    fn griglia_mese_di_sei_righe() {
        // Agosto 2026 parte sabato (colonna 5) e ha 31 giorni: il 31 finisce da solo nella sesta riga.
        let g = griglia(2026, 8);
        assert_eq!(g[0], [0, 0, 0, 0, 0, 1, 2]);
        assert_eq!(g[5], [31, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn navigazione_mese() {
        let dic = Mese::new(2025, 12).unwrap();
        assert_eq!(dic.successivo(), Mese { anno: 2026, mese: 1 });
        let gen = Mese::new(2026, 1).unwrap();
        assert_eq!(gen.precedente(), Mese { anno: 2025, mese: 12 });
        assert_eq!(gen.successivo(), Mese { anno: 2026, mese: 2 });
        assert_eq!(Mese::new(2026, 13), None);
    }

    #[test]
    fn aggiungi_ordina_per_data_e_ora() {
        let mut c = Calendario::new();
        assert!(c.aggiungi(b"dentista", d(2026, 5, 2), o(15, 0)));
        assert!(c.aggiungi(b"pranzo", d(2026, 5, 2), o(12, 30)));
        assert!(c.aggiungi(b"prima", d(2026, 5, 1), o(23, 59)));
        assert_eq!(c.count(), 3);
        assert_eq!(c.evento(0).unwrap().titolo(), b"prima");
        assert_eq!(c.evento(1).unwrap().titolo(), b"pranzo");
        assert_eq!(c.evento(2).unwrap().titolo(), b"dentista");
        assert_eq!(c.evento(3), None);
    }

    #[test]
    fn aggiungi_rifiuta_casi_errati() {
        let mut c = Calendario::new();
        assert!(!c.aggiungi(b"", d(2026, 5, 2), o(9, 0)));
        assert!(!c.aggiungi(b"0123456789abcdefX", d(2026, 5, 2), o(9, 0)));
        assert!(c.aggiungi(b"0123456789abcdef", d(2026, 5, 2), o(9, 0)));
        assert_eq!(Ora::new(24, 0), None);
        assert_eq!(Ora::new(0, 60), None);
        assert_eq!(c.count(), 1);
    }

    #[test]
    fn limite_sedici_eventi() {
        let mut c = Calendario::new();
        for i in 0..MAX_EVENTI {
            assert!(c.aggiungi(b"x", d(2026, 1, 1), o(i as u8, 0)));
        }
        assert!(!c.aggiungi(b"x", d(2026, 1, 1), o(23, 0)));
        assert_eq!(c.count(), MAX_EVENTI);
    }

    #[test]
    fn rimuovi_evento() {
        let mut c = Calendario::new();
        c.aggiungi(b"a", d(2026, 1, 1), o(8, 0));
        c.aggiungi(b"b", d(2026, 1, 2), o(8, 0));
        c.aggiungi(b"c", d(2026, 1, 3), o(8, 0));
        assert!(c.rimuovi(1));
        assert_eq!(c.count(), 2);
        assert_eq!(c.evento(0).unwrap().titolo(), b"a");
        assert_eq!(c.evento(1).unwrap().titolo(), b"c");
        assert!(!c.rimuovi(2));
    }

    #[test]
    fn eventi_di_un_giorno() {
        let mut c = Calendario::new();
        c.aggiungi(b"sera", d(2026, 3, 4), o(20, 0));
        c.aggiungi(b"altro giorno", d(2026, 3, 5), o(7, 0));
        c.aggiungi(b"mattina", d(2026, 3, 4), o(7, 45));
        let titoli: [&[u8]; 2] = {
            let mut it = c.del_giorno(d(2026, 3, 4));
            [it.next().unwrap().titolo(), it.next().unwrap().titolo()]
        };
        assert_eq!(titoli, [&b"mattina"[..], &b"sera"[..]]);
        assert_eq!(c.del_giorno(d(2026, 3, 6)).count(), 0);
    }
}
