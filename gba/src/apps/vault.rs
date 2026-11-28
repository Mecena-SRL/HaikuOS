//! Cifratura del gestore password (`passwords::Archivio`). Solo `core`, niente heap.
//! Formato: `"HPW1"` (4) | sale (16) | nonce (16) | `ciphertext || tag` (Ascon-AEAD128).
//! Intestazione = dati associati (autenticata). Il tag rifiuta password sbagliata e file alterato.
//! Nessun verificatore in chiaro: l'unico test della password è il tag.
//! Il nonce non deve ripetersi con la stessa chiave: il chiamante passa un contatore a ogni salvataggio.
//! KDF provvisoria: `ITER` giri di Ascon-AEAD (stretching). Costo su ARM7TDMI: da verificare.

#![allow(dead_code)]

use super::passwords::{Archivio, DIM_MAX};
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
}
