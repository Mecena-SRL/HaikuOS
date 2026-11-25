//! Microkernel swap nel simulatore: pagina cifrata e compressa (slot.rs) su un blocco dello storage.
//! Ordine: comprimere, poi cifrare (docs/memory-security.md). Nessun hardware reale.
//! Il blocco è a dimensione fissa: i primi 2 byte dicono la lunghezza del dato cifrato.

use crate::crypto::{CryptoError, Vault};
use crate::resource::{Arch, Kind, Manifest};
use crate::storage::{Device, StorageError};

pub const SWAP_MANIFEST: Manifest = Manifest {
    name: "swap-lzss-ascon",
    kind: Kind::Swap,
    archs: &[Arch::Gba, Arch::Arm64, Arch::X86_64],
    required: false,
    bytes: 2 * 1024,
    queue_depth: 4,
};

pub const PAGE_BYTES: usize = 256;
const BLOCK_BYTES: usize = 2 + 320;

#[derive(Debug, PartialEq, Eq)]
pub enum SwapError {
    Crypto(CryptoError),
    Storage(StorageError),
    /// La pagina cifrata non entra nel blocco (dati casuali si espandono).
    TooLarge,
    NoFreeBlock,
}

pub struct Swap {
    dev: Device,
    vault: Vault,
    free: Vec<usize>,
}

impl Swap {
    pub fn new(blocks: usize, vault: Vault) -> Self {
        Self { dev: Device::new(blocks, BLOCK_BYTES), vault, free: (0..blocks).rev().collect() }
    }

    /// Scrive una pagina nello swap. Ritorna il blocco usato.
    pub fn page_out(&mut self, page: &[u8]) -> Result<usize, SwapError> {
        let sealed = self.vault.seal(0, page).map_err(SwapError::Crypto)?;
        if sealed.len() + 2 > BLOCK_BYTES {
            return Err(SwapError::TooLarge);
        }
        let blk = self.free.pop().ok_or(SwapError::NoFreeBlock)?;
        let mut data = vec![0u8; BLOCK_BYTES];
        data[..2].copy_from_slice(&(sealed.len() as u16).to_le_bytes());
        data[2..2 + sealed.len()].copy_from_slice(&sealed);
        self.dev.write(blk, &data).map_err(SwapError::Storage)?;
        Ok(blk)
    }

    /// Rilegge una pagina. Un blocco alterato fallisce per checksum o per cifratura.
    pub fn page_in(&mut self, blk: usize) -> Result<Vec<u8>, SwapError> {
        let raw = self.dev.read(blk).map_err(SwapError::Storage)?;
        let len = u16::from_le_bytes([raw[0], raw[1]]) as usize;
        let sealed = raw.get(2..2 + len).ok_or(SwapError::TooLarge)?.to_vec();
        self.vault.open(0, &sealed).map_err(SwapError::Crypto)
    }

    pub fn free_page(&mut self, blk: usize) {
        self.free.push(blk);
    }

    /// Solo per i test: altera un byte del blocco, come un guasto reale.
    pub fn inject_fault(&mut self, blk: usize, at: usize) {
        self.dev.inject_fault(blk, at);
    }
}

use crate::resource::Microkernel;

impl Microkernel for Swap {
    fn manifest(&self) -> crate::resource::Manifest {
        SWAP_MANIFEST.clone()
    }
}
