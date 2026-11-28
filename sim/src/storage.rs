//! Microkernel storage nel simulatore: blocchi a dimensione fissa con checksum per blocco.
//! Un blocco alterato si rileva alla lettura. Nessun hardware reale.
//! Regole: docs/resource-microkernels.md; metadati: docs/filesystem-roadmap.md.

use crate::resource::{Arch, Kind, Manifest, Microkernel, Status};

/// Manifest del driver storage generico: funziona su tutte le architetture con blocchi.
pub const STORAGE_MANIFEST: Manifest = Manifest {
    name: "storage-generic",
    kind: Kind::Storage,
    archs: &[Arch::Gba, Arch::Arm64, Arch::X86_64],
    required: true,
    bytes: 2 * 1024,
    queue_depth: 4,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageError {
    OutOfRange,
    BadSize,
    /// Checksum non corrispondente: il blocco è alterato.
    Corrupt,
    /// Microkernel fermo: niente scritture. Le letture restano.
    Halted,
}

/// FNV-1a a 32 bit. Solo per rilevare alterazioni accidentali, non come firma.
pub fn checksum(data: &[u8]) -> u32 {
    data.iter().fold(0x811c_9dc5u32, |h, &b| (h ^ b as u32).wrapping_mul(0x0100_0193))
}

pub struct Device {
    running: bool,
    block_bytes: usize,
    blocks: Vec<Vec<u8>>,
    sums: Vec<u32>,
}

impl Device {
    pub fn new(block_count: usize, block_bytes: usize) -> Self {
        let zero = vec![0u8; block_bytes];
        Self {
            running: true,
            block_bytes,
            blocks: vec![zero.clone(); block_count],
            sums: vec![checksum(&zero); block_count],
        }
    }

    /// Ferma le scritture. Le letture restano per svuotare il disco.
    pub fn halt(&mut self) {
        self.running = false;
    }

    pub fn write(&mut self, idx: usize, data: &[u8]) -> Result<(), StorageError> {
        if !self.running {
            return Err(StorageError::Halted);
        }
        if idx >= self.blocks.len() {
            return Err(StorageError::OutOfRange);
        }
        if data.len() != self.block_bytes {
            return Err(StorageError::BadSize);
        }
        self.blocks[idx].copy_from_slice(data);
        self.sums[idx] = checksum(data);
        Ok(())
    }

    pub fn read(&self, idx: usize) -> Result<&[u8], StorageError> {
        let b = self.blocks.get(idx).ok_or(StorageError::OutOfRange)?;
        if checksum(b) != self.sums[idx] {
            return Err(StorageError::Corrupt);
        }
        Ok(b)
    }

    /// Solo per i test: altera un byte senza aggiornare il checksum, come un guasto reale.
    pub fn inject_fault(&mut self, idx: usize, at: usize) {
        self.blocks[idx][at] ^= 0xff;
    }
}

impl Microkernel for Device {
    fn manifest(&self) -> Manifest {
        STORAGE_MANIFEST
    }
    fn status(&self) -> Status {
        if self.running { Status::Ready } else { Status::Stopped }
    }
}
