//! Cifratura del gestore password (`passwords::Archivio`). Solo `core`, niente heap.
//! Formato: `"HPW1"` (4) | sale (16) | nonce (16) | `ciphertext || tag` (Ascon-AEAD128).
//! Intestazione = dati associati (autenticata). Il tag rifiuta password sbagliata e file alterato.
//! Nessun verificatore in chiaro: l'unico test della password è il tag.
//! Il nonce non deve ripetersi con la stessa chiave: il chiamante passa un contatore a ogni salvataggio.
//! KDF provvisoria: `ITER` giri di Ascon-AEAD (stretching). Costo su ARM7TDMI: da verificare.

#![allow(dead_code)]

use super::passwords::{Archivio, Errore, DIM_MAX};
use crate::ascon;

pub const MAGIC: &[u8; 4] = b"HPW1";
pub const INTESTAZIONE: usize = 4 + 16 + 16;
pub const TAG: usize = 16;
/// Giri di stretching della password. Da misurare sul GBA.
pub const ITER: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErroreVault {
    /// Dati troppo corti, magic sbagliato o archivio interno non valido.
    Formato,
    /// Password sbagliata o file alterato (tag non valido).
    Password,
    /// Buffer di output troppo corto.
    BufferCorto,
}

/// Chiave da password e sale: `ITER` giri di Ascon-AEAD, ogni giro usa la password come dato associato.
fn deriva(password: &[u8], sale: &[u8; 16]) -> [u8; 16] {
    let mut k = *sale;
    let mut tag = [0u8; 16];
    for _ in 0..ITER {
        ascon::seal(&k, sale, password, &[], &mut tag);
        k = tag;
    }
    tag.fill(0);
    k
}

/// Cifra l'archivio in `out`. Ritorna i byte scritti.
/// `sale` e `nonce` devono essere nuovi a ogni salvataggio (il nonce è un contatore, il sale casuale o fisso per vault).
pub fn cifra(
    a: &Archivio,
    password: &[u8],
    sale: &[u8; 16],
    nonce: &[u8; 16],
    out: &mut [u8],
) -> Result<usize, ErroreVault> {
    let n = a.lunghezza_serializzata();
    let tot = INTESTAZIONE + n + TAG;
    if out.len() < tot {
        return Err(ErroreVault::BufferCorto);
    }
    let mut clear = [0u8; DIM_MAX];
    a.serializza(&mut clear).map_err(|_| ErroreVault::Formato)?;
    let mut head = [0u8; INTESTAZIONE];
    head[..4].copy_from_slice(MAGIC);
    head[4..20].copy_from_slice(sale);
    head[20..36].copy_from_slice(nonce);
    let mut k = deriva(password, sale);
    out[..INTESTAZIONE].copy_from_slice(&head);
    let w = ascon::seal(&k, nonce, &head, &clear[..n], &mut out[INTESTAZIONE..tot]);
    clear.fill(0);
    k.fill(0);
    if w == 0 {
        return Err(ErroreVault::BufferCorto);
    }
    Ok(tot)
}

/// Decifra `src` dentro `out`. Con password sbagliata `out` resta invariato.
pub fn apri(src: &[u8], password: &[u8], out: &mut Archivio) -> Result<(), ErroreVault> {
    if src.len() < INTESTAZIONE + TAG || src.len() - INTESTAZIONE - TAG > DIM_MAX || &src[..4] != MAGIC {
        return Err(ErroreVault::Formato);
    }
    let mut sale = [0u8; 16];
    let mut nonce = [0u8; 16];
    sale.copy_from_slice(&src[4..20]);
    nonce.copy_from_slice(&src[20..36]);
    let mut k = deriva(password, &sale);
    let mut clear = [0u8; DIM_MAX];
    let r = ascon::open(&k, &nonce, &src[..INTESTAZIONE], &src[INTESTAZIONE..], &mut clear);
    k.fill(0);
    let res = match r {
        None => Err(ErroreVault::Password),
        Some(n) => out.carica(&clear[..n]).map_err(|_| ErroreVault::Formato),
    };
    clear.fill(0);
    res
}

/// Nome del file nello spazio utente.
pub const NOME: &[u8] = b"vault";
/// Dimensione massima del file cifrato.
pub const CAP: usize = INTESTAZIONE + DIM_MAX + TAG;
const ALFABETO: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!@#$%&*-_.+ 0123456789";
/// Lunghezza massima della master password.
pub const PW_MAX: usize = 16;

/// Campo di inserimento della master password: su/giù cambia il carattere, A lo aggiunge, Select cancella.
#[derive(Clone, Copy)]
pub struct Ingresso {
    buf: [u8; PW_MAX],
    len: usize,
    cur: usize,
}

impl Ingresso {
    pub const fn new() -> Self {
        Ingresso { buf: [0; PW_MAX], len: 0, cur: 0 }
    }
    pub fn giu(&mut self) {
        self.cur = (self.cur + 1) % ALFABETO.len();
    }
    pub fn su(&mut self) {
        self.cur = (self.cur + ALFABETO.len() - 1) % ALFABETO.len();
    }
    /// Aggiunge il carattere corrente. `false` se il campo è pieno.
    pub fn aggiungi(&mut self) -> bool {
        if self.len == PW_MAX {
            return false;
        }
        self.buf[self.len] = ALFABETO[self.cur];
        self.len += 1;
        true
    }
    pub fn cancella(&mut self) {
        if self.len > 0 {
            self.len -= 1;
            self.buf[self.len] = 0;
        }
    }
    pub fn password(&self) -> &[u8] {
        &self.buf[..self.len]
    }
    pub fn lunghezza(&self) -> usize {
        self.len
    }
    /// Carattere selezionato in questo momento.
    pub fn carattere(&self) -> u8 {
        ALFABETO[self.cur]
    }
    pub fn azzera(&mut self) {
        self.buf.fill(0);
        self.len = 0;
    }
}

/// Sale casuale per una nuova cassaforte, dal seme di sistema (PRF Ascon). Unico per vault, non segreto.
pub fn sale_da_seme(seme: u64) -> [u8; 16] {
    let mut k = [0u8; 16];
    k[..8].copy_from_slice(&seme.to_le_bytes());
    let mut tag = [0u8; 16];
    ascon::seal(&k, &[0; 16], b"haikuos-sale", &[], &mut tag);
    tag
}

/// Sale e nonce dall'intestazione di un file cifrato (senza verificarlo).
pub fn intestazione_di(src: &[u8]) -> Option<([u8; 16], [u8; 16])> {
    if src.len() < INTESTAZIONE || &src[..4] != MAGIC {
        return None;
    }
    let (mut sale, mut nonce) = ([0u8; 16], [0u8; 16]);
    sale.copy_from_slice(&src[4..20]);
    nonce.copy_from_slice(&src[20..36]);
    Some((sale, nonce))
}

fn u64_da(n: &[u8; 16]) -> u64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&n[..8]);
    u64::from_le_bytes(b)
}

/// Un campo di una voce in composizione.
#[derive(Clone, Copy)]
pub struct Campo {
    dati: [u8; PW_MAX],
    len: usize,
}

const CAMPO_VUOTO: Campo = Campo { dati: [0; PW_MAX], len: 0 };

impl Campo {
    pub fn bytes(&self) -> &[u8] {
        &self.dati[..self.len]
    }
}

/// Voce in composizione: tre campi (servizio, utente, segreto) e il campo corrente.
/// `indice` è la voce da sostituire, `None` per una nuova.
#[derive(Clone, Copy)]
pub struct Modifica {
    pub campi: [Campo; 3],
    pub campo: usize,
    pub indice: Option<usize>,
}

/// Stato della cassaforte nella finestra password.
pub struct Cassa {
    pub aperta: bool,
    pub ingresso: Ingresso,
    pub archivio: Archivio,
    /// Esito dell'ultimo tentativo (vuoto se nessun errore).
    pub esito: &'static [u8],
    /// File cifrato da scrivere in memoria, se `da_salvare`.
    pub blob: [u8; CAP],
    pub blob_len: usize,
    pub da_salvare: bool,
    /// Seme per il sale di una nuova cassaforte (impostato dal sistema a ogni passo).
    pub seme: u64,
    /// Voce in composizione, se c'è.
    pub modifica: Option<Modifica>,
    /// Master password dell'archivio aperto, per risalvarlo. Azzerata alla chiusura.
    master: Ingresso,
    sale: [u8; 16],
    /// Contatore dei salvataggi: il nonce non si ripete mai con la stessa chiave.
    contatore: u64,
}

impl Cassa {
    pub const fn new() -> Self {
        Cassa {
            aperta: false,
            ingresso: Ingresso::new(),
            archivio: Archivio::new(),
            esito: b"",
            blob: [0; CAP],
            blob_len: 0,
            da_salvare: false,
            seme: 0,
            modifica: None,
            master: Ingresso::new(),
            sale: [0; 16],
            contatore: 0,
        }
    }

    /// Sblocca con la password inserita. `file` è il contenuto salvato, o `None` se non esiste ancora:
    /// in quel caso crea un archivio vuoto cifrato con questa password.
    pub fn sblocca(&mut self, file: Option<&[u8]>) {
        match file {
            None => {
                self.sale = sale_da_seme(self.seme);
                self.contatore = 0;
                self.archivio = Archivio::new();
                self.master = self.ingresso;
                if self.salva() {
                    self.aperta = true;
                    self.esito = b"";
                } else {
                    self.esito = b"ERRORE CREAZIONE";
                }
            }
            Some(bytes) => match intestazione_di(bytes) {
                None => self.esito = b"ARCHIVIO NON VALIDO",
                Some((sale, nonce)) => match apri(bytes, self.ingresso.password(), &mut self.archivio) {
                    Ok(()) => {
                        self.sale = sale;
                        self.contatore = u64_da(&nonce).wrapping_add(1);
                        self.master = self.ingresso;
                        self.aperta = true;
                        self.esito = b"";
                    }
                    Err(ErroreVault::Password) => self.esito = b"PASSWORD ERRATA",
                    Err(_) => self.esito = b"ARCHIVIO NON VALIDO",
                },
            },
        }
        self.ingresso.azzera();
    }

    /// Cifra l'archivio attuale in `blob` con un nuovo nonce. `false` se fallisce.
    fn salva(&mut self) -> bool {
        let mut nonce = [0u8; 16];
        nonce[..8].copy_from_slice(&self.contatore.to_le_bytes());
        match cifra(&self.archivio, self.master.password(), &self.sale, &nonce, &mut self.blob) {
            Ok(n) => {
                self.blob_len = n;
                self.da_salvare = true;
                self.contatore = self.contatore.wrapping_add(1);
                true
            }
            Err(_) => false,
        }
    }

    /// Avvia la composizione di una voce nuova (`None`) o da sostituire (`Some(indice)`).
    pub fn inizia(&mut self, indice: Option<usize>) {
        self.modifica = Some(Modifica { campi: [CAMPO_VUOTO; 3], campo: 0, indice });
        self.ingresso.azzera();
        self.esito = b"";
    }

    /// Annulla la composizione in corso.
    pub fn annulla(&mut self) {
        self.modifica = None;
        self.ingresso.azzera();
    }

    /// Conferma il campo corrente. Dopo l'ultimo campo inserisce o sostituisce la voce e salva.
    pub fn avanti(&mut self) {
        let Some(mut m) = self.modifica else { return };
        let pw = self.ingresso.password();
        let mut c = CAMPO_VUOTO;
        c.dati[..pw.len()].copy_from_slice(pw);
        c.len = pw.len();
        m.campi[m.campo] = c;
        m.campo += 1;
        self.ingresso.azzera();
        if m.campo < 3 {
            self.modifica = Some(m);
            return;
        }
        let [s, u, p] = &m.campi;
        let r = match m.indice {
            None => self.archivio.aggiungi(s.bytes(), u.bytes(), p.bytes()).map(|_| ()),
            Some(i) => self.archivio.sostituisci(i, s.bytes(), u.bytes(), p.bytes()),
        };
        self.modifica = None;
        self.esito = match r {
            Ok(()) => {
                if self.salva() {
                    b""
                } else {
                    b"ERRORE SALVATAGGIO"
                }
            }
            Err(Errore::Pieno) => b"ARCHIVIO PIENO",
            Err(Errore::ServizioVuoto) => b"SERVIZIO VUOTO",
            Err(_) => b"VOCE NON VALIDA",
        };
    }

    /// Chiude l'archivio e azzera i dati in chiaro.
    pub fn chiudi(&mut self) {
        self.aperta = false;
        self.archivio = Archivio::new();
        self.ingresso.azzera();
        self.master.azzera();
        self.modifica = None;
        self.esito = b"";
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn esempio() -> Archivio {
        let mut a = Archivio::new();
        a.aggiungi(b"Mail", b"anna", b"segreto1").unwrap();
        a.aggiungi(b"Banca", b"anna.rossi", b"1234").unwrap();
        a
    }

    const SALE: [u8; 16] = [7; 16];
    const NONCE: [u8; 16] = [9; 16];

    #[test]
    fn cifra_e_apri_tornano_uguali() {
        let a = esempio();
        let mut buf = [0u8; INTESTAZIONE + DIM_MAX + TAG];
        let n = cifra(&a, b"correct horse", &SALE, &NONCE, &mut buf).unwrap();
        assert_eq!(n, INTESTAZIONE + a.lunghezza_serializzata() + TAG);
        let mut b = Archivio::new();
        assert_eq!(apri(&buf[..n], b"correct horse", &mut b), Ok(()));
        assert_eq!(b.len(), 2);
        assert_eq!(b.cerca(b"banca"), Some(1));
        assert_eq!(b.voce(1).unwrap().segreto(), b"1234");
    }

    #[test]
    fn nessun_testo_in_chiaro() {
        let a = esempio();
        let mut buf = [0u8; INTESTAZIONE + DIM_MAX + TAG];
        let n = cifra(&a, b"pw", &SALE, &NONCE, &mut buf).unwrap();
        let cerca = |needle: &[u8]| buf[..n].windows(needle.len()).any(|w| w == needle);
        assert!(!cerca(b"segreto1"));
        assert!(!cerca(b"Banca"));
        assert!(!cerca(b"anna"));
    }

    #[test]
    fn password_sbagliata_rifiutata_senza_dati() {
        let a = esempio();
        let mut buf = [0u8; INTESTAZIONE + DIM_MAX + TAG];
        let n = cifra(&a, b"giusta", &SALE, &NONCE, &mut buf).unwrap();
        let mut b = esempio();
        assert_eq!(apri(&buf[..n], b"sbagliata", &mut b), Err(ErroreVault::Password));
        assert_eq!(b.len(), 2);
        assert_eq!(b.voce(0).unwrap().segreto(), b"segreto1");
    }

    #[test]
    fn file_alterato_rifiutato() {
        let a = esempio();
        let mut buf = [0u8; INTESTAZIONE + DIM_MAX + TAG];
        let n = cifra(&a, b"pw", &SALE, &NONCE, &mut buf).unwrap();
        // Byte di cifrato alterato.
        buf[INTESTAZIONE + 3] ^= 1;
        let mut b = Archivio::new();
        assert_eq!(apri(&buf[..n], b"pw", &mut b), Err(ErroreVault::Password));
        buf[INTESTAZIONE + 3] ^= 1;
        // Nonce alterato (è nell'intestazione, quindi autenticato).
        buf[25] ^= 1;
        assert_eq!(apri(&buf[..n], b"pw", &mut b), Err(ErroreVault::Password));
        buf[25] ^= 1;
        // Magic sbagliato.
        buf[0] = b'X';
        assert_eq!(apri(&buf[..n], b"pw", &mut b), Err(ErroreVault::Formato));
    }

    #[test]
    fn formato_corto_e_buffer_corto() {
        let mut b = Archivio::new();
        assert_eq!(apri(&[0u8; 10], b"pw", &mut b), Err(ErroreVault::Formato));
        let a = esempio();
        let mut piccolo = [0u8; 20];
        assert_eq!(cifra(&a, b"pw", &SALE, &NONCE, &mut piccolo), Err(ErroreVault::BufferCorto));
    }

    #[test]
    fn sale_diverso_altra_chiave() {
        let a = esempio();
        let mut buf = [0u8; INTESTAZIONE + DIM_MAX + TAG];
        let n = cifra(&a, b"pw", &SALE, &NONCE, &mut buf).unwrap();
        let mut altro = [0u8; INTESTAZIONE + DIM_MAX + TAG];
        let m = cifra(&a, b"pw", &[1; 16], &NONCE, &mut altro).unwrap();
        assert_ne!(&buf[INTESTAZIONE..n], &altro[INTESTAZIONE..m]);
    }

    #[test]
    fn ingresso_cicla_e_cancella() {
        let mut i = Ingresso::new();
        assert_eq!(i.carattere(), b'A');
        i.su();
        assert_eq!(i.carattere(), b'9');
        i.giu();
        assert!(i.aggiungi());
        i.giu();
        assert!(i.aggiungi());
        assert_eq!(i.password(), b"AB");
        i.cancella();
        assert_eq!(i.password(), b"A");
        for _ in 0..PW_MAX {
            i.aggiungi();
        }
        assert_eq!(i.lunghezza(), PW_MAX);
        assert!(!i.aggiungi());
        i.azzera();
        assert_eq!(i.lunghezza(), 0);
    }

    fn digita(c: &mut Cassa, pw: &[u8]) {
        c.ingresso.azzera();
        for &ch in pw {
            while c.ingresso.carattere() != ch {
                c.ingresso.giu();
            }
            c.ingresso.aggiungi();
        }
    }

    #[test]
    fn voce_inserita_e_modificata_persistono() {
        let mut c = Cassa::new();
        c.seme = 42;
        digita(&mut c, b"K1");
        c.sblocca(None);
        c.inizia(None);
        digita(&mut c, b"Mail");
        c.avanti();
        digita(&mut c, b"anna");
        c.avanti();
        digita(&mut c, b"seg1");
        c.avanti();
        assert_eq!(c.archivio.len(), 1);
        assert!(c.da_salvare && c.esito.is_empty());
        c.inizia(Some(0));
        digita(&mut c, b"Banca");
        c.avanti();
        digita(&mut c, b"x");
        c.avanti();
        digita(&mut c, b"1234");
        c.avanti();
        assert_eq!(c.archivio.voce(0).unwrap().servizio(), b"Banca");
        let salvato = c.blob[..c.blob_len].to_vec();
        c.chiudi();
        digita(&mut c, b"K1");
        c.sblocca(Some(&salvato));
        assert!(c.aperta);
        assert_eq!(c.archivio.voce(0).unwrap().segreto(), b"1234");
    }

    #[test]
    fn sale_diverso_per_seme_diverso() {
        assert_ne!(sale_da_seme(1), sale_da_seme(2));
        let mut c = Cassa::new();
        c.seme = 7;
        digita(&mut c, b"K1");
        c.sblocca(None);
        assert_eq!(intestazione_di(&c.blob[..c.blob_len]).unwrap().0, sale_da_seme(7));
    }

    #[test]
    fn cassa_crea_poi_sblocca_con_stessa_password() {
        let mut c = Cassa::new();
        digita(&mut c, b"ABC1");
        c.sblocca(None);
        assert!(c.aperta && c.da_salvare);
        let salvato = c.blob[..c.blob_len].to_vec();
        c.chiudi();
        assert!(!c.aperta);
        digita(&mut c, b"ABC2");
        c.sblocca(Some(&salvato));
        assert!(!c.aperta);
        assert_eq!(c.esito, b"PASSWORD ERRATA");
        digita(&mut c, b"ABC1");
        c.sblocca(Some(&salvato));
        assert!(c.aperta);
        assert_eq!(c.archivio.len(), 0);
        assert!(c.ingresso.password().is_empty());
    }
}
