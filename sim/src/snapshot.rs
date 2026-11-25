//! Snapshot leggeri a copy-on-write sui blocchi, cifrati (simulazione host, spazio utente).
//! Uno snapshot copia solo la tabella file -> voce (metadati), non i dati: i blocchi sono condivisi
//! e contati. Una scrittura non modifica mai un blocco: ne alloca uno nuovo.
//! Ogni blocco contiene LZSS poi Ascon-AEAD (vedi `slot.rs`): l'AAD lega il dato al suo indice di blocco.
//! Limiti di partenza: da verificare (vedi docs/filesystem-roadmap.md).
use std::collections::BTreeMap;

use crate::slot;

/// Blocchi totali dello store simulato. Da verificare.
pub const SNAP_BLOCKS: usize = 64;
/// Dimensione di un blocco fisico, cifratura inclusa (nonce 16 + tag 16). Da verificare.
pub const SNAP_BLOCK_BYTES: usize = 128;
/// Dimensione massima di un file in chiaro. Da verificare.
pub const SNAP_MAX_FILE: usize = 256;
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

/// Voce di un file: blocco, byte occupati nel blocco, lunghezza in chiaro, checksum.
#[derive(Clone, Copy)]
struct Meta {
    blk: usize,
    stored: usize,
    len: usize,
    sum: u32,
}

/// File -> voce. Stessa forma per la vista viva e per gli snapshot.
type Table = BTreeMap<String, Meta>;

fn fnv_with(mut h: u32, b: &[u8]) -> u32 {
    for &x in b {
        h ^= x as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

/// Checksum dei metadati di una voce (nome, blocco, lunghezze). Da verificare su GBA.
fn meta_sum(name: &str, m: &Meta) -> u32 {
    let mut nums = [0u8; 12];
    nums[0..4].copy_from_slice(&(m.blk as u32).to_le_bytes());
    nums[4..8].copy_from_slice(&(m.stored as u32).to_le_bytes());
    nums[8..12].copy_from_slice(&(m.len as u32).to_le_bytes());
    fnv_with(fnv_with(0x811c_9dc5, name.as_bytes()), &nums)
}

fn seal_meta(name: &str, mut m: Meta) -> Meta {
    m.sum = meta_sum(name, &m);
    m
}

/// Controlla il checksum di ogni voce di una tabella.
fn check_table(t: &Table) -> Result<(), SnapError> {
    for (name, m) in t {
        if m.sum != meta_sum(name, m) {
            return Err(SnapError::Corrupt);
        }
    }
    Ok(())
}

pub struct Store {
    key: [u8; 16],
    /// Contatore per i nonce: ogni scrittura ne usa uno nuovo.
    nonce_ctr: u64,
    blocks: Vec<[u8; SNAP_BLOCK_BYTES]>,
    /// Riferimenti per blocco: 0 = libero.
    refs: Vec<u32>,
    files: Table,
    snaps: Vec<(String, Table)>,
}

impl Store {
    pub fn new(key: [u8; 16]) -> Self {
        Self {
            key,
            nonce_ctr: 0,
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

    /// Scrive `data` in `name` su un blocco nuovo, comprimendo e cifrando.
    pub fn write(&mut self, name: &str, data: &[u8]) -> Result<(), SnapError> {
        if data.len() > SNAP_MAX_FILE {
            return Err(SnapError::TooLarge);
        }
        let b = self.alloc()?;
        let mut nonce = [0u8; 16];
        nonce[8..].copy_from_slice(&self.nonce_ctr.to_le_bytes());
        self.nonce_ctr += 1;
        let sealed = match slot::seal(data, &self.key, &nonce, b as u8, SNAP_BLOCK_BYTES) {
            Ok(s) => s,
            Err(_) => {
                self.release(b);
                return Err(SnapError::TooLarge);
            }
        };
        self.blocks[b][..sealed.len()].copy_from_slice(&sealed);
        let m = seal_meta(name, Meta { blk: b, stored: sealed.len(), len: data.len(), sum: 0 });
        if let Some(old) = self.files.insert(name.to_string(), m) {
            self.release(old.blk);
        }
        Ok(())
    }

    fn unpack(&self, m: &Meta) -> Result<Vec<u8>, SnapError> {
        let raw = &self.blocks[m.blk][..m.stored];
        slot::open(raw, &self.key, m.blk as u8, SNAP_MAX_FILE).map_err(|_| SnapError::Corrupt)
    }

    /// Lettura con verifica del checksum della voce e dell'autenticazione. `Err(Corrupt)` se alterata.
    pub fn read_checked(&self, name: &str) -> Result<Option<Vec<u8>>, SnapError> {
        match self.files.get(name) {
            None => Ok(None),
            Some(m) if m.sum == meta_sum(name, m) => self.unpack(m).map(Some),
            Some(_) => Err(SnapError::Corrupt),
        }
    }

    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.read_checked(name).ok().flatten()
    }

    pub fn delete(&mut self, name: &str) -> bool {
        match self.files.remove(name) {
            Some(m) => {
                self.release(m.blk);
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
        for m in self.files.values() {
            self.refs[m.blk] += 1;
        }
        self.snaps.push((label.to_string(), self.files.clone()));
        Ok(())
    }

    /// Verifica i checksum di tutte le voci, vivo e snapshot.
    pub fn verify(&self) -> Result<(), SnapError> {
        check_table(&self.files)?;
        self.snaps.iter().try_for_each(|(_, t)| check_table(t))
    }

    /// Torna alla tabella dello snapshot. Lo snapshot resta disponibile.
    pub fn rollback(&mut self, label: &str) -> Result<(), SnapError> {
        let i = self.snaps.iter().position(|(l, _)| l == label).ok_or(SnapError::NoSuchSnapshot)?;
        let target = self.snaps[i].1.clone();
        // Si verifica prima di toccare qualsiasi refcount: un errore non cambia lo stato.
        check_table(&target)?;
        // Prima si incrementa il target, poi si rilascia il vivo: i blocchi condivisi non si liberano.
        for m in target.values() {
            self.refs[m.blk] += 1;
        }
        let live = std::mem::replace(&mut self.files, target);
        for m in live.values() {
            self.release(m.blk);
        }
        Ok(())
    }

    /// Elimina lo snapshot e libera i blocchi che nessun altro usa.
    pub fn drop_snapshot(&mut self, label: &str) -> Result<(), SnapError> {
        let i = self.snaps.iter().position(|(l, _)| l == label).ok_or(SnapError::NoSuchSnapshot)?;
        let (_, table) = self.snaps.remove(i);
        for m in table.values() {
            self.release(m.blk);
        }
        Ok(())
    }

    /// Solo per i test: altera il blocco di una voce viva senza aggiornarne il checksum.
    #[doc(hidden)]
    pub fn tamper_meta(&mut self, name: &str, new_blk: usize) {
        if let Some(m) = self.files.get_mut(name) {
            m.blk = new_blk;
        }
    }

    /// Solo per i test: altera il blocco di una voce di uno snapshot.
    #[doc(hidden)]
    pub fn tamper_snapshot(&mut self, label: &str, name: &str, new_blk: usize) {
        if let Some((_, t)) = self.snaps.iter_mut().find(|(l, _)| l == label) {
            if let Some(m) = t.get_mut(name) {
                m.blk = new_blk;
            }
        }
    }

    /// Solo per i test: indice del blocco di una voce viva.
    #[doc(hidden)]
    pub fn files_block(&self, name: &str) -> usize {
        self.files[name].blk
    }

    /// Solo per i test: corrompe un byte cifrato di un blocco (rilevato dall'autenticazione).
    #[doc(hidden)]
    pub fn flip_block_byte(&mut self, blk: usize) {
        self.blocks[blk][20] ^= 1;
    }
}
