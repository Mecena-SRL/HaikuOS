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
    Corrupt,
}

/// File -> (blocco, lunghezza, checksum dei metadati). Stessa forma per la vista viva e per gli snapshot.
type Table = BTreeMap<String, (usize, usize, u32)>;

/// Checksum FNV-1a dei metadati di un file (nome, blocco, lunghezza). Da verificare su GBA.
fn meta_sum(name: &str, blk: usize, len: usize) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for &x in name.as_bytes().iter().chain(&(blk as u32).to_le_bytes()).chain(&(len as u32).to_le_bytes()) {
        h ^= x as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

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
        if let Some((old, _, _)) = self.files.insert(name.to_string(), (b, data.len(), meta_sum(name, b, data.len()))) {
            self.release(old);
        }
        Ok(())
    }

    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.files.get(name).map(|&(b, len, _)| self.blocks[b][..len].to_vec())
    }

    /// Lettura con verifica del checksum dei metadati. `Err(Corrupt)` se la voce è alterata.
    pub fn read_checked(&self, name: &str) -> Result<Option<Vec<u8>>, SnapError> {
        match self.files.get(name) {
            None => Ok(None),
            Some(&(b, len, sum)) if sum == meta_sum(name, b, len) => Ok(Some(self.blocks[b][..len].to_vec())),
            Some(_) => Err(SnapError::Corrupt),
        }
    }

    /// Solo per i test: altera il blocco di una voce senza aggiornarne il checksum.
    #[doc(hidden)]
    pub fn tamper_meta(&mut self, name: &str, new_blk: usize) {
        if let Some(e) = self.files.get_mut(name) {
            e.0 = new_blk;
        }
    }

    pub fn delete(&mut self, name: &str) -> bool {
        match self.files.remove(name) {
            Some((b, _, _)) => {
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
        for &(b, _, _) in self.files.values() {
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
        for &(b, _, _) in target.values() {
            self.refs[b] += 1;
        }
        let live = std::mem::replace(&mut self.files, target);
        for &(b, _, _) in live.values() {
            self.release(b);
        }
        Ok(())
    }

    /// Elimina lo snapshot e libera i blocchi che nessun altro usa.
    pub fn drop_snapshot(&mut self, label: &str) -> Result<(), SnapError> {
        let i = self.snaps.iter().position(|(l, _)| l == label).ok_or(SnapError::NoSuchSnapshot)?;
        let (_, table) = self.snaps.remove(i);
        for &(b, _, _) in table.values() {
            self.release(b);
        }
        Ok(())
    }
}
