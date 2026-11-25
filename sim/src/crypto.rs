//! Microkernel crittografia nel simulatore: chiave per slot, nonce a contatore mai riusato.
//! Usa `slot.rs` (LZSS + Ascon-AEAD128). Nessun hardware reale.
//! Regole: docs/memory-security.md, docs/resource-microkernels.md.

use crate::resource::{Arch, Kind, Manifest};
use crate::slot::{self, SlotError};

/// Manifest generico: Ascon ovunque, come da decisione in docs/memory-security.md.
pub const CRYPTO_MANIFEST: Manifest = Manifest {
    name: "crypto-ascon",
    kind: Kind::Crypto,
    archs: &[Arch::Gba, Arch::Arm64, Arch::X86_64],
    required: true,
    bytes: 2 * 1024,
    queue_depth: 4,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CryptoError {
    Slot(SlotError),
    UnknownSlot,
}

pub struct Vault {
    key: [u8; 16],
    /// Un contatore per slot: il nonce non si riusa mai con la stessa chiave.
    counters: Vec<u64>,
    slot_bytes: usize,
}

impl Vault {
    pub fn new(key: [u8; 16], slots: usize, slot_bytes: usize) -> Self {
        Self { key, counters: vec![0; slots], slot_bytes }
    }

    fn nonce(slot: u8, counter: u64) -> [u8; 16] {
        let mut n = [0u8; 16];
        n[..8].copy_from_slice(&counter.to_le_bytes());
        n[8] = slot;
        n
    }

    /// Cifra per uno slot e incrementa il contatore. Il nonce dipende da slot e contatore.
    pub fn seal(&mut self, slot_id: u8, plain: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let c = self.counters.get_mut(slot_id as usize).ok_or(CryptoError::UnknownSlot)?;
        let nonce = Self::nonce(slot_id, *c);
        *c += 1;
        slot::seal(plain, &self.key, &nonce, slot_id, self.slot_bytes).map_err(CryptoError::Slot)
    }

    /// Apre un blocco: se è di un altro slot o alterato, fallisce.
    pub fn open(&self, slot_id: u8, sealed: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if slot_id as usize >= self.counters.len() {
            return Err(CryptoError::UnknownSlot);
        }
        slot::open(sealed, &self.key, slot_id, self.slot_bytes).map_err(CryptoError::Slot)
    }
}
