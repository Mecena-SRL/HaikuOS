//! Gestore password: archivio fisso di al più 8 voci (servizio, utente, segreto), ogni campo al più 16 byte.
//! Logica pura: niente disegno, niente heap, solo `core`.
//! Cifratura: NON qui. cifratura con gba/src/ascon.rs: da integrare.
//! Serializzazione: byte 0 = numero di voci; per ogni voce tre campi da 17 byte
//! (1 byte di lunghezza + 16 di dati, riempiti con zeri). Massimo `DIM_MAX` byte.

#![allow(dead_code)]

pub const MAX_VOCI: usize = 8;
pub const MAX_CAMPO: usize = 16;
const CAMPO_BYTE: usize = 1 + MAX_CAMPO;
const VOCE_BYTE: usize = 3 * CAMPO_BYTE;
/// Dimensione massima della serializzazione: 1 + 8 * 51 = 409 byte.
pub const DIM_MAX: usize = 1 + MAX_VOCI * VOCE_BYTE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Errore {
    /// Archivio pieno.
    Pieno,
    /// Un campo supera 16 byte.
    CampoLungo,
    /// Il nome del servizio è vuoto.
    ServizioVuoto,
    /// Indice fuori dall'archivio.
    NonTrovato,
    /// Buffer di serializzazione troppo corto.
    BufferCorto,
    /// Dati da caricare non validi.
    Formato,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Campo {
    dati: [u8; MAX_CAMPO],
    len: u8,
}

impl Campo {
    const VUOTO: Campo = Campo { dati: [0; MAX_CAMPO], len: 0 };

    fn da(s: &[u8]) -> Result<Campo, Errore> {
        if s.len() > MAX_CAMPO {
            return Err(Errore::CampoLungo);
        }
        let mut c = Campo::VUOTO;
        c.dati[..s.len()].copy_from_slice(s);
        c.len = s.len() as u8;
        Ok(c)
    }

    fn byte(&self) -> &[u8] {
        &self.dati[..self.len as usize]
    }
}

/// Una voce dell'archivio.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Voce {
    servizio: Campo,
    utente: Campo,
    segreto: Campo,
}

const VOCE_VUOTA: Voce = Voce { servizio: Campo::VUOTO, utente: Campo::VUOTO, segreto: Campo::VUOTO };

impl Voce {
    pub fn new(servizio: &[u8], utente: &[u8], segreto: &[u8]) -> Result<Voce, Errore> {
        if servizio.is_empty() {
            return Err(Errore::ServizioVuoto);
        }
        Ok(Voce {
            servizio: Campo::da(servizio)?,
            utente: Campo::da(utente)?,
            segreto: Campo::da(segreto)?,
        })
    }

    pub fn servizio(&self) -> &[u8] {
        self.servizio.byte()
    }

    pub fn utente(&self) -> &[u8] {
        self.utente.byte()
    }

    pub fn segreto(&self) -> &[u8] {
        self.segreto.byte()
    }
}

/// Archivio di al più `MAX_VOCI` voci, con una selezione (su/giù).
pub struct Archivio {
    voci: [Voce; MAX_VOCI],
    len: usize,
    cur: usize,
}

impl Archivio {
    pub const fn new() -> Self {
        Archivio { voci: [VOCE_VUOTA; MAX_VOCI], len: 0, cur: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Indice della voce selezionata (0 se l'archivio è vuoto).
    pub fn selezionata(&self) -> usize {
        self.cur
    }

    /// Voce all'indice `i`, se esiste.
    pub fn voce(&self, i: usize) -> Option<&Voce> {
        if i < self.len { Some(&self.voci[i]) } else { None }
    }

    /// Voce selezionata, se l'archivio non è vuoto.
    pub fn voce_selezionata(&self) -> Option<&Voce> {
        self.voce(self.cur)
    }

    /// Aggiunge una voce in coda e restituisce il suo indice.
    pub fn aggiungi(&mut self, servizio: &[u8], utente: &[u8], segreto: &[u8]) -> Result<usize, Errore> {
        if self.len == MAX_VOCI {
            return Err(Errore::Pieno);
        }
        self.voci[self.len] = Voce::new(servizio, utente, segreto)?;
        self.len += 1;
        Ok(self.len - 1)
    }

    /// Rimuove la voce `i` spostando le successive. La selezione resta valida.
    pub fn rimuovi(&mut self, i: usize) -> Result<(), Errore> {
        if i >= self.len {
            return Err(Errore::NonTrovato);
        }
        for k in i..self.len - 1 {
            self.voci[k] = self.voci[k + 1];
        }
        self.len -= 1;
        self.voci[self.len] = VOCE_VUOTA;
        if self.cur >= self.len {
            self.cur = self.len.saturating_sub(1);
        }
        Ok(())
    }

    /// Sostituisce la voce `i`. `NonTrovato` se `i` non esiste; altri errori se i campi non sono validi.
    pub fn sostituisci(&mut self, i: usize, servizio: &[u8], utente: &[u8], segreto: &[u8]) -> Result<(), Errore> {
        if i >= self.len {
            return Err(Errore::NonTrovato);
        }
        self.voci[i] = Voce::new(servizio, utente, segreto)?;
        Ok(())
    }

    /// Indice della prima voce con questo servizio (maiuscole ignorate).
    pub fn cerca(&self, servizio: &[u8]) -> Option<usize> {
        (0..self.len).find(|&i| self.voci[i].servizio().eq_ignore_ascii_case(servizio))
    }

    /// Seleziona la voce `i`. `false` se non esiste.
    pub fn seleziona(&mut self, i: usize) -> bool {
        if i >= self.len {
            return false;
        }
        self.cur = i;
        true
    }

    /// Voce precedente, senza giro. `false` se è già la prima.
    pub fn su(&mut self) -> bool {
        if self.cur == 0 || self.cur >= self.len {
            return false;
        }
        self.cur -= 1;
        true
    }

    /// Voce successiva, senza giro. `false` se è già l'ultima.
    pub fn giu(&mut self) -> bool {
        if self.cur + 1 >= self.len {
            return false;
        }
        self.cur += 1;
        true
    }

    /// Byte necessari per la serializzazione dello stato attuale.
    pub fn lunghezza_serializzata(&self) -> usize {
        1 + self.len * VOCE_BYTE
    }

    /// Scrive l'archivio in `out` e restituisce i byte usati. `BufferCorto` se `out` è più piccolo.
    pub fn serializza(&self, out: &mut [u8]) -> Result<usize, Errore> {
        let need = self.lunghezza_serializzata();
        if out.len() < need {
            return Err(Errore::BufferCorto);
        }
        out[0] = self.len as u8;
        for i in 0..self.len {
            let base = 1 + i * VOCE_BYTE;
            let v = &self.voci[i];
            scrivi_campo(&v.servizio, &mut out[base..base + CAMPO_BYTE]);
            scrivi_campo(&v.utente, &mut out[base + CAMPO_BYTE..base + 2 * CAMPO_BYTE]);
            scrivi_campo(&v.segreto, &mut out[base + 2 * CAMPO_BYTE..base + VOCE_BYTE]);
        }
        Ok(need)
    }

    /// Legge un archivio serializzato. In caso di errore lo stato resta invariato.
    pub fn carica(&mut self, src: &[u8]) -> Result<(), Errore> {
        let n = *src.first().ok_or(Errore::Formato)? as usize;
        if n > MAX_VOCI || src.len() < 1 + n * VOCE_BYTE {
            return Err(Errore::Formato);
        }
        let mut voci = [VOCE_VUOTA; MAX_VOCI];
        for (i, slot) in voci.iter_mut().enumerate().take(n) {
            let base = 1 + i * VOCE_BYTE;
            let servizio = leggi_campo(&src[base..base + CAMPO_BYTE])?;
            if servizio.len == 0 {
                return Err(Errore::Formato);
            }
            *slot = Voce {
                servizio,
                utente: leggi_campo(&src[base + CAMPO_BYTE..base + 2 * CAMPO_BYTE])?,
                segreto: leggi_campo(&src[base + 2 * CAMPO_BYTE..base + VOCE_BYTE])?,
            };
        }
        self.voci = voci;
        self.len = n;
        self.cur = 0;
        Ok(())
    }
}

fn scrivi_campo(c: &Campo, out: &mut [u8]) {
    out.fill(0);
    out[0] = c.len;
    out[1..1 + c.len as usize].copy_from_slice(c.byte());
}

fn leggi_campo(src: &[u8]) -> Result<Campo, Errore> {
    let l = src[0] as usize;
    if l > MAX_CAMPO {
        return Err(Errore::Formato);
    }
    let mut c = Campo::VUOTO;
    c.dati[..l].copy_from_slice(&src[1..1 + l]);
    c.len = l as u8;
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn esempio() -> Archivio {
        let mut a = Archivio::new();
        a.aggiungi(b"Mail", b"anna", b"segreto1").unwrap();
        a.aggiungi(b"Banca", b"anna.rossi", b"1234").unwrap();
        a.aggiungi(b"Forum", b"ann", b"x").unwrap();
        a
    }

    #[test]
    fn aggiungi_e_pieno() {
        let mut a = Archivio::new();
        assert!(a.is_empty());
        for i in 0..MAX_VOCI {
            assert_eq!(a.aggiungi(b"s", b"u", b"p"), Ok(i));
        }
        assert_eq!(a.len(), MAX_VOCI);
        assert_eq!(a.aggiungi(b"s", b"u", b"p"), Err(Errore::Pieno));
        assert_eq!(a.len(), MAX_VOCI);
    }

    #[test]
    fn campi_troppo_lunghi_o_servizio_vuoto() {
        let mut a = Archivio::new();
        assert_eq!(a.aggiungi(b"s", b"0123456789abcdefX", b"p"), Err(Errore::CampoLungo));
        assert_eq!(a.aggiungi(b"s", b"u", b"0123456789abcdefX"), Err(Errore::CampoLungo));
        assert_eq!(a.aggiungi(b"", b"u", b"p"), Err(Errore::ServizioVuoto));
        assert!(a.is_empty());
        // 16 byte esatti sono ammessi.
        assert!(a.aggiungi(b"s", b"0123456789abcdef", b"").is_ok());
        assert_eq!(a.voce(0).unwrap().utente(), b"0123456789abcdef");
    }

    #[test]
    fn rimuovi_sposta_e_tiene_selezione() {
        let mut a = esempio();
        assert!(a.seleziona(2));
        assert_eq!(a.rimuovi(0), Ok(()));
        assert_eq!(a.len(), 2);
        assert_eq!(a.voce(0).unwrap().servizio(), b"Banca");
        assert_eq!(a.selezionata(), 1);
        assert_eq!(a.rimuovi(5), Err(Errore::NonTrovato));
        assert_eq!(a.rimuovi(1), Ok(()));
        assert_eq!(a.rimuovi(0), Ok(()));
        assert!(a.is_empty());
        assert_eq!(a.selezionata(), 0);
        assert!(a.voce_selezionata().is_none());
    }

    #[test]
    fn cerca_ignora_maiuscole() {
        let a = esempio();
        assert_eq!(a.cerca(b"banca"), Some(1));
        assert_eq!(a.cerca(b"BANCA"), Some(1));
        assert_eq!(a.cerca(b"Banc"), None);
        assert_eq!(a.cerca(b"nessuno"), None);
    }

    #[test]
    fn selezione_su_giu_senza_giro() {
        let mut a = esempio();
        assert!(!a.su());
        assert!(a.giu());
        assert!(a.giu());
        assert!(!a.giu());
        assert_eq!(a.selezionata(), 2);
        assert!(a.su());
        assert_eq!(a.voce_selezionata().unwrap().servizio(), b"Banca");
        assert!(!a.seleziona(3));
        let mut vuoto = Archivio::new();
        assert!(!vuoto.giu());
        assert!(!vuoto.su());
    }

    #[test]
    fn serializza_e_carica_tornano_uguali() {
        let a = esempio();
        let mut buf = [0u8; DIM_MAX];
        let n = a.serializza(&mut buf).unwrap();
        assert_eq!(n, 1 + 3 * VOCE_BYTE);
        assert_eq!(n, a.lunghezza_serializzata());
        let mut b = Archivio::new();
        b.carica(&buf[..n]).unwrap();
        assert_eq!(b.len(), 3);
        for i in 0..3 {
            let (x, y) = (a.voce(i).unwrap(), b.voce(i).unwrap());
            assert!(x == y);
        }
        assert_eq!(b.cerca(b"forum"), Some(2));
    }

    #[test]
    fn serializza_archivio_pieno_entra_in_dim_max() {
        let mut a = Archivio::new();
        for _ in 0..MAX_VOCI {
            a.aggiungi(b"0123456789abcdef", b"0123456789abcdef", b"0123456789abcdef").unwrap();
        }
        let mut buf = [0u8; DIM_MAX];
        assert_eq!(a.serializza(&mut buf), Ok(DIM_MAX));
        let mut b = Archivio::new();
        assert_eq!(b.carica(&buf), Ok(()));
        assert_eq!(b.len(), MAX_VOCI);
    }

    #[test]
    fn buffer_corto_e_errore() {
        let a = esempio();
        let mut buf = [0u8; 10];
        assert_eq!(a.serializza(&mut buf), Err(Errore::BufferCorto));
        let vuoto = Archivio::new();
        let mut un_byte = [0u8; 1];
        assert_eq!(vuoto.serializza(&mut un_byte), Ok(1));
        assert_eq!(un_byte[0], 0);
    }

    #[test]
    fn carica_dati_non_validi_lascia_stato() {
        let mut a = esempio();
        assert_eq!(a.carica(&[]), Err(Errore::Formato));
        assert_eq!(a.carica(&[9]), Err(Errore::Formato));
        assert_eq!(a.carica(&[1, 5, b'a']), Err(Errore::Formato));
        // Lunghezza di campo oltre 16.
        let mut bad = [0u8; 1 + VOCE_BYTE];
        bad[0] = 1;
        bad[1] = 17;
        assert_eq!(a.carica(&bad), Err(Errore::Formato));
        // Servizio vuoto.
        let mut vuoto_serv = [0u8; 1 + VOCE_BYTE];
        vuoto_serv[0] = 1;
        assert_eq!(a.carica(&vuoto_serv), Err(Errore::Formato));
        assert_eq!(a.len(), 3);
        assert_eq!(a.cerca(b"Mail"), Some(0));
    }
}
