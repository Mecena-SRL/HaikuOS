//! Pila di strati per richieste a blocchi (idea GEOM, riscritta da zero).
//! Ogni strato implementa lo stesso trait `Block` e avvolge quello sotto: offset di partizione,
//! sola lettura. Nessun hardware reale. Regole: docs/kernel-study.md, riga 20.

use crate::storage::{Device, StorageError};

/// Interfaccia comune di blocchi: uno strato o un disco reale.
pub trait Block {
    fn block_bytes(&self) -> usize;
    fn blocks(&self) -> usize;
    fn read(&self, idx: usize, out: &mut [u8]) -> Result<(), StorageError>;
    fn write(&mut self, idx: usize, data: &[u8]) -> Result<(), StorageError>;
}

impl Block for Device {
    fn block_bytes(&self) -> usize {
        Device::block_bytes(self)
    }
    fn blocks(&self) -> usize {
        Device::block_count(self)
    }
    fn read(&self, idx: usize, out: &mut [u8]) -> Result<(), StorageError> {
        let src = Device::read(self, idx)?;
        if out.len() != src.len() {
            return Err(StorageError::BadSize);
        }
        out.copy_from_slice(src);
        Ok(())
    }
    fn write(&mut self, idx: usize, data: &[u8]) -> Result<(), StorageError> {
        Device::write(self, idx, data)
    }
}

/// Strato partizione: espone solo `len` blocchi a partire da `base`.
pub struct Offset<B: Block> {
    inner: B,
    base: usize,
    len: usize,
}

impl<B: Block> Offset<B> {
    /// `None` se la finestra esce dal disco sotto.
    pub fn new(inner: B, base: usize, len: usize) -> Option<Self> {
        let end = base.checked_add(len)?;
        (end <= inner.blocks()).then_some(Self { inner, base, len })
    }
}

impl<B: Block> Block for Offset<B> {
    fn block_bytes(&self) -> usize {
        self.inner.block_bytes()
    }
    fn blocks(&self) -> usize {
        self.len
    }
    fn read(&self, idx: usize, out: &mut [u8]) -> Result<(), StorageError> {
        if idx >= self.len {
            return Err(StorageError::OutOfRange);
        }
        self.inner.read(self.base + idx, out)
    }
    fn write(&mut self, idx: usize, data: &[u8]) -> Result<(), StorageError> {
        if idx >= self.len {
            return Err(StorageError::OutOfRange);
        }
        self.inner.write(self.base + idx, data)
    }
}

/// Strato sola lettura: rifiuta ogni scrittura senza toccare il disco sotto.
pub struct ReadOnly<B: Block>(pub B);

impl<B: Block> Block for ReadOnly<B> {
    fn block_bytes(&self) -> usize {
        self.0.block_bytes()
    }
    fn blocks(&self) -> usize {
        self.0.blocks()
    }
    fn read(&self, idx: usize, out: &mut [u8]) -> Result<(), StorageError> {
        self.0.read(idx, out)
    }
    fn write(&mut self, _: usize, _: &[u8]) -> Result<(), StorageError> {
        Err(StorageError::Halted)
    }
}
