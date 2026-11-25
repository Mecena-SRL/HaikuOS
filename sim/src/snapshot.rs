//! Snapshot leggeri a copy-on-write sui blocchi (simulazione host, spazio utente).
//! Uno snapshot copia solo la tabella file -> blocco (metadati), non i dati: i blocchi
//! sono condivisi e contati. Una scrittura non modifica mai un blocco: ne alloca uno nuovo.
//! Limiti di partenza: da verificare (vedi docs/filesystem-roadmap.md).
use std::collections::BTreeMap;

/// Blocchi totali dello store simulato. Da verificare.
pub const SNAP_BLOCKS: usize = 64;
/// Dimensione di un blocco; un file occupa al massimo un blocco. Da verificare.
pub const SNAP_BLOCK_BYTES: usize = 64;
/// Snapshot contemporanei massimi. Da verificare.
pub const MAX_SNAPSHOTS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapError {
    NoSpace,
    TooLarge,
    TooManySnapshots,
    NoSuchSnapshot,
}

/// File -> (blocco, lunghezza). Stessa forma per la vista viva e per gli snapshot.
type Table = BTreeMap<String, (usize, usize)>;

pub struct Store {
    blocks: Vec<[u8; SNAP_BLOCK_BYTES]>,
    /// Riferimenti per blocco: 0 = libero.
    refs: Vec<u32>,
    files: Table,
    snaps: Vec<(String, Table)>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            blocks: vec![[0; SNAP_BLOCK_BYTES]; SNAP_BLOCKS],
            refs: vec![0; SNAP_BLOCKS],
            files: Table::new(),
            snaps: Vec::new(),
        }
    }

    pub fn free_blocks(&self) -> usize {
        self.refs.iter().filter(|&&r| r == 0).count()
    }

    fn alloc(&mut self) -> Result<usize, SnapError> {
        let b = self.refs.iter().position(|&r| r == 0).ok_or(SnapError::NoSpace)?;
        self.refs[b] = 1;
        Ok(b)
    }

    fn release(&mut self, b: usize) {
        self.refs[b] -= 1;
    }

    /// Scrive `data` in `name` su un blocco nuovo; il vecchio resta vivo se uno snapshot lo usa.
    pub fn write(&mut self, name: &str, data: &[u8]) -> Result<(), SnapError> {
        if data.len() > SNAP_BLOCK_BYTES {
            return Err(SnapError::TooLarge);
        }
        let b = self.alloc()?;
        self.blocks[b][..data.len()].copy_from_slice(data);
        if let Some((old, _)) = self.files.insert(name.to_string(), (b, data.len())) {
            self.release(old);
        }
        Ok(())
    }

    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.files.get(name).map(|&(b, len)| self.blocks[b][..len].to_vec())
    }

    pub fn delete(&mut self, name: &str) -> bool {
        match self.files.remove(name) {
            Some((b, _)) => {
                self.release(b);
                true
            }
            None => false,
        }
    }

    /// Fotografa la tabella corrente. Nessun dato viene copiato.
    pub fn snapshot(&mut self, label: &str) -> Result<(), SnapError> {
        if self.snaps.len() >= MAX_SNAPSHOTS {
            return Err(SnapError::TooManySnapshots);
        }
        for &(b, _) in self.files.values() {
            self.refs[b] += 1;
        }
        self.snaps.push((label.to_string(), self.files.clone()));
        Ok(())
    }

    /// Torna alla tabella dello snapshot. Lo snapshot resta disponibile.
    pub fn rollback(&mut self, label: &str) -> Result<(), SnapError> {
        let i = self.snaps.iter().position(|(l, _)| l == label).ok_or(SnapError::NoSuchSnapshot)?;
        let target = self.snaps[i].1.clone();
        // Prima si incrementa il target, poi si rilascia il vivo: i blocchi condivisi non si liberano.
        for &(b, _) in target.values() {
            self.refs[b] += 1;
        }
        let live = std::mem::replace(&mut self.files, target);
        for &(b, _) in live.values() {
            self.release(b);
        }
        Ok(())
    }

    /// Elimina lo snapshot e libera i blocchi che nessun altro usa.
    pub fn drop_snapshot(&mut self, label: &str) -> Result<(), SnapError> {
        let i = self.snaps.iter().position(|(l, _)| l == label).ok_or(SnapError::NoSuchSnapshot)?;
        let (_, table) = self.snaps.remove(i);
        for &(b, _) in table.values() {
            self.release(b);
        }
        Ok(())
    }
}
