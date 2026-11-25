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
/// Scritture massime per blocco prima di ritirarlo (endurance). Valore segnaposto, da verificare.
pub const DEFAULT_ENDURANCE: u32 = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapError {
    NoSpace,
    TooLarge,
    TooManySnapshots,
    NoSuchSnapshot,
    Corrupt,
    NoSuchFile,
    /// Restano solo blocchi liberi ma esauriti (endurance raggiunta).
    Worn,
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
    /// Contatore di riscritture per blocco (un'allocazione = una riscrittura).
    wear: Vec<u32>,
    endurance: u32,
    /// Contatore per i nonce: ogni scrittura ne usa uno nuovo.
    nonce_ctr: u64,
    blocks: Vec<[u8; SNAP_BLOCK_BYTES]>,
    /// Seconda copia di ogni blocco (mirror). Ogni copia si autentica da sola con Ascon.
    mirror: Vec<[u8; SNAP_BLOCK_BYTES]>,
    /// Byte cifrati occupati in ogni blocco.
    stored_len: Vec<usize>,
    /// Riferimenti per blocco: 0 = libero.
    refs: Vec<u32>,
    files: Table,
    snaps: Vec<(String, Table)>,
}

impl Store {
    pub fn new(key: [u8; 16]) -> Self {
        Self::with_endurance(key, DEFAULT_ENDURANCE)
    }

    pub fn with_endurance(key: [u8; 16], endurance: u32) -> Self {
        Self {
            key,
            wear: vec![0; SNAP_BLOCKS],
            endurance,
            nonce_ctr: 0,
            blocks: vec![[0; SNAP_BLOCK_BYTES]; SNAP_BLOCKS],
            mirror: vec![[0; SNAP_BLOCK_BYTES]; SNAP_BLOCKS],
            stored_len: vec![0; SNAP_BLOCKS],
            refs: vec![0; SNAP_BLOCKS],
            files: Table::new(),
            snaps: Vec::new(),
        }
    }

    pub fn free_blocks(&self) -> usize {
        self.refs.iter().filter(|&&r| r == 0).count()
    }

    /// Riscritture fatte su un blocco.
    pub fn wear(&self, blk: usize) -> u32 {
        self.wear[blk]
    }

    /// Sceglie il blocco libero meno usato (wear leveling) e lo conta come riscritto.
    fn alloc(&mut self) -> Result<usize, SnapError> {
        let free = (0..SNAP_BLOCKS).filter(|&b| self.refs[b] == 0);
        let best = free.clone().filter(|&b| self.wear[b] < self.endurance).min_by_key(|&b| (self.wear[b], b));
        match best {
            Some(b) => {
                self.refs[b] = 1;
                self.wear[b] += 1;
                Ok(b)
            }
            None if free.count() > 0 => Err(SnapError::Worn),
            None => Err(SnapError::NoSpace),
        }
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
        self.mirror[b][..sealed.len()].copy_from_slice(&sealed);
        self.stored_len[b] = sealed.len();
        let m = seal_meta(name, Meta { blk: b, stored: sealed.len(), len: data.len(), sum: 0 });
        if let Some(old) = self.files.insert(name.to_string(), m) {
            self.release(old.blk);
        }
        Ok(())
    }

    /// Apre una copia di un blocco; `None` se l'autenticazione fallisce.
    fn open_copy(&self, blk: usize, copy: &[u8; SNAP_BLOCK_BYTES], len: usize) -> Option<Vec<u8>> {
        slot::open(&copy[..len], &self.key, blk as u8, SNAP_MAX_FILE).ok()
    }

    /// Legge da una copia, se valida, altrimenti dal mirror.
    fn unpack(&self, m: &Meta) -> Result<Vec<u8>, SnapError> {
        self.open_copy(m.blk, &self.blocks[m.blk], m.stored)
            .or_else(|| self.open_copy(m.blk, &self.mirror[m.blk], m.stored))
            .ok_or(SnapError::Corrupt)
    }

    /// Verifica le copie di ogni blocco in uso e ripara quella danneggiata dall'altra.
    /// Ritorna quante copie sono state riparate. `Err(Corrupt)` se entrambe sono perse.
    pub fn scrub(&mut self) -> Result<usize, SnapError> {
        let mut repaired = 0;
        for b in 0..SNAP_BLOCKS {
            if self.refs[b] == 0 {
                continue;
            }
            let len = self.stored_len[b];
            let p_ok = self.open_copy(b, &self.blocks[b], len).is_some();
            let m_ok = self.open_copy(b, &self.mirror[b], len).is_some();
            match (p_ok, m_ok) {
                (true, true) => {}
                (false, true) => {
                    self.blocks[b] = self.mirror[b];
                    repaired += 1;
                }
                (true, false) => {
                    self.mirror[b] = self.blocks[b];
                    repaired += 1;
                }
                (false, false) => return Err(SnapError::Corrupt),
            }
        }
        Ok(repaired)
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

    /// Crea `dst` come copia di `src` senza copiare dati: i due nomi condividono il blocco.
    /// Una scrittura successiva su uno dei due alloca un blocco nuovo (copy-on-write).
    pub fn clone_file(&mut self, src: &str, dst: &str) -> Result<(), SnapError> {
        let m = *self.files.get(src).ok_or(SnapError::NoSuchFile)?;
        self.refs[m.blk] += 1;
        let m = seal_meta(dst, m);
        if let Some(old) = self.files.insert(dst.to_string(), m) {
            self.release(old.blk);
        }
        Ok(())
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

    /// Solo per i test: corrompe un byte della copia mirror di un blocco.
    #[doc(hidden)]
    pub fn flip_mirror_byte(&mut self, blk: usize) {
        self.mirror[blk][20] ^= 1;
    }
}
