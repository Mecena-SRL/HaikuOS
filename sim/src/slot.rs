//! Uno slot cifrato: LZSS poi Ascon-AEAD128. Formato: nonce(16) || ciphertext || tag(16).
//! I dati associati (AAD) legano il blocco allo slot, così non si scambiano slot.
//! Da verificare: nessun test su hardware; il nonce va gestito dal monolite.

use crate::lzss::{self, LzssError};
use ascon_aead::aead::{Aead, KeyInit, Payload};
use ascon_aead::AsconAead128;

pub const NONCE_BYTES: usize = 16;
pub const TAG_BYTES: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotError {
    /// Il blocco cifrato supererebbe lo slot.
    TooLarge,
    /// Tag non valido: dati alterati, chiave o slot sbagliati.
    Auth,
    Corrupt,
}

impl From<LzssError> for SlotError {
    fn from(e: LzssError) -> Self {
        match e {
            LzssError::TooLarge => SlotError::TooLarge,
            LzssError::Corrupt => SlotError::Corrupt,
        }
    }
}

fn aad(slot: u8) -> [u8; 5] {
    [b'H', b'K', b'S', b'L', slot]
}

/// Comprime, cifra e controlla che il risultato entri in `slot_bytes`.
pub fn seal(plain: &[u8], key: &[u8; 16], nonce: &[u8; 16], slot: u8, slot_bytes: usize) -> Result<Vec<u8>, SlotError> {
    let packed = lzss::compress(plain);
    let cipher = AsconAead128::new(&(*key).into());
    let ct = cipher
        .encrypt(&(*nonce).into(), Payload { msg: &packed, aad: &aad(slot) })
        .map_err(|_| SlotError::Auth)?;
    let mut out = Vec::with_capacity(NONCE_BYTES + ct.len());
    out.extend_from_slice(nonce);
    out.extend_from_slice(&ct);
    if out.len() > slot_bytes {
        return Err(SlotError::TooLarge);
    }
    Ok(out)
}

/// Verifica il tag, decifra e decomprime. Il risultato non supera `slot_bytes`.
pub fn open(sealed: &[u8], key: &[u8; 16], slot: u8, slot_bytes: usize) -> Result<Vec<u8>, SlotError> {
    if sealed.len() < NONCE_BYTES + TAG_BYTES {
        return Err(SlotError::Corrupt);
    }
    let (nonce, ct) = sealed.split_at(NONCE_BYTES);
    let nonce: [u8; NONCE_BYTES] = nonce.try_into().map_err(|_| SlotError::Corrupt)?;
    let cipher = AsconAead128::new(&(*key).into());
    let packed = cipher
        .decrypt(&nonce.into(), Payload { msg: ct, aad: &aad(slot) })
        .map_err(|_| SlotError::Auth)?;
    Ok(lzss::decompress(&packed, slot_bytes)?)
}
