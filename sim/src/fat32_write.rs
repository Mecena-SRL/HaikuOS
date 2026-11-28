//! Scrittura FAT32 nel simulatore, dietro il flag `fat32-write`. Cluster da 512 byte, nomi 8.3.
//! Ordine di scrittura: dati, FAT (entrambe le copie), voce di directory per ultima: è il punto di commit.
//! Una directory occupa un cluster solo (max 16 voci). **Da verificare** come il lettore (`fat32.rs`).
//! Controllo: `check` trova cluster allocati e non raggiungibili (persi) e cluster condivisi.

use crate::fat32::{Fat32, FatError, FAT32_MIN_CLUSTERS, DirEntry};

const BPS: usize = 512;
const RSVD: usize = 32;
const NFATS: usize = 2;
const EOC: u32 = 0x0FFF_FFFF;
const ATTR_FILE: u8 = 0x20;
const ATTR_DIR: u8 = 0x10;
const ROOT: u32 = 2;

fn put(v: &mut [u8], o: usize, b: &[u8]) {
    v[o..o + b.len()].copy_from_slice(b);
}
fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

/// Formatta `vol` come FAT32 con 1 settore per cluster. Richiede abbastanza cluster per essere FAT32.
pub fn format(vol: &mut [u8]) -> Result<(), FatError> {
    let tot = vol.len() / BPS;
    if tot < 30 {
        return Err(FatError::TooSmall);
    }
    // Minimo s tale che la FAT (s settori) copra tutti i cluster: (tot - 30) * 4 <= 520 * s.
    let fatsz = ((tot - 30) * 4).div_ceil(520);
    let clusters = tot.checked_sub(RSVD + NFATS * fatsz).ok_or(FatError::TooSmall)?;
    if clusters < FAT32_MIN_CLUSTERS as usize {
        return Err(FatError::NotFat32);
    }
    vol[..(RSVD + NFATS * fatsz + 1) * BPS].fill(0);
    put(vol, 0, &[0xEB, 0x58, 0x90]);
    put(vol, 3, b"MSWIN4.1");
    put(vol, 11, &(BPS as u16).to_le_bytes());
    vol[13] = 1;
    put(vol, 14, &(RSVD as u16).to_le_bytes());
    vol[16] = NFATS as u8;
    vol[21] = 0xF8;
    put(vol, 32, &(tot as u32).to_le_bytes());
    put(vol, 36, &(fatsz as u32).to_le_bytes());
    put(vol, 44, &ROOT.to_le_bytes());
    put(vol, 48, &1u16.to_le_bytes());
    put(vol, 510, &[0x55, 0xAA]);
    for copy in 0..NFATS {
        let base = (RSVD + copy * fatsz) * BPS;
        put(vol, base, &0x0FFF_FFF8u32.to_le_bytes());
        put(vol, base + 4, &EOC.to_le_bytes());
        put(vol, base + 8, &EOC.to_le_bytes());
    }
    Ok(())
}

/// Geometria del volume, letta dal boot sector dopo la validazione del lettore.
struct Geo {
    fat1: usize,
    fat2: usize,
    data: usize,
    clusters: u32,
}

impl Geo {
    fn of(vol: &[u8]) -> Result<Geo, FatError> {
        let (bps, spc, fat1, clusters) = {
            let fs = Fat32::open(vol)?;
            fs.geometry()
        };
        if bps != BPS || spc != 1 {
            return Err(FatError::BadBpb);
        }
        let fatsz = u32_at(vol, 36) as usize;
        Ok(Geo {
            fat1,
            fat2: fat1 + fatsz * BPS,
            data: fat1 + NFATS * fatsz * BPS,
            clusters,
        })
    }
    fn cl_off(&self, c: u32) -> Result<usize, FatError> {
        if c < 2 || c - 2 >= self.clusters {
            return Err(FatError::BadCluster);
        }
        Ok(self.data + (c as usize - 2) * BPS)
    }
    fn get(&self, vol: &[u8], c: u32) -> u32 {
        u32_at(vol, self.fat1 + c as usize * 4) & 0x0FFF_FFFF
    }
    fn set(&self, vol: &mut [u8], c: u32, val: u32) {
        for base in [self.fat1, self.fat2] {
            put(vol, base + c as usize * 4, &val.to_le_bytes());
        }
    }
    /// Primi `n` cluster liberi (voce FAT = 0). Non scrive nulla.
    fn alloc(&self, vol: &[u8], n: usize) -> Result<Vec<u32>, FatError> {
        let mut free = Vec::with_capacity(n);
        for c in 3..self.clusters + 2 {
            if free.len() == n {
                break;
            }
            if self.get(vol, c) == 0 {
                free.push(c);
            }
        }
        if free.len() < n {
            return Err(FatError::NoSpace);
        }
        Ok(free)
    }
    /// Scrive `data` nei cluster di `chain` e li concatena nella FAT (entrambe le copie).
    fn write_chain(&self, vol: &mut [u8], chain: &[u32], data: &[u8]) -> Result<(), FatError> {
        for (i, &c) in chain.iter().enumerate() {
            let off = self.cl_off(c)?;
            let chunk = &data[(i * BPS).min(data.len())..((i + 1) * BPS).min(data.len())];
            vol[off..off + BPS].fill(0);
            put(vol, off, chunk);
        }
        for (i, &c) in chain.iter().enumerate() {
            let val = chain.get(i + 1).copied().unwrap_or(EOC);
            self.set(vol, c, val);
        }
        Ok(())
    }
    /// Primo slot libero della directory `dir` (un cluster). Errore se il nome esiste già.
    fn slot(&self, vol: &[u8], dir: u32, name: &[u8; 11]) -> Result<usize, FatError> {
        let base = self.cl_off(dir)?;
        let mut free = None;
        for k in 0..BPS / 32 {
            let o = base + k * 32;
            let e = &vol[o..o + 32];
            if e[0] == 0x00 {
                free.get_or_insert(o);
                break;
            }
            if e[0] == 0xE5 {
                free.get_or_insert(o);
            } else if e[11] & 0x0F != 0x0F && e[11] & 0x08 == 0 && &e[0..11] == name {
                return Err(FatError::Exists);
            }
        }
        free.ok_or(FatError::DirFull)
    }
}

fn dir_entry(name: &[u8; 11], attr: u8, first: u32, size: u32) -> [u8; 32] {
    let mut e = [0u8; 32];
    e[0..11].copy_from_slice(name);
    e[11] = attr;
    e[20..22].copy_from_slice(&((first >> 16) as u16).to_le_bytes());
    e[26..28].copy_from_slice(&((first & 0xFFFF) as u16).to_le_bytes());
    e[28..32].copy_from_slice(&size.to_le_bytes());
    e
}

/// Crea un file `name` (8.3) dentro la directory `dir`.
pub fn create_file_in(vol: &mut [u8], dir: u32, name: &[u8; 11], data: &[u8]) -> Result<(), FatError> {
    let g = Geo::of(vol)?;
    let slot = g.slot(vol, dir, name)?;
    let chain = g.alloc(vol, data.len().div_ceil(BPS))?;
    g.write_chain(vol, &chain, data)?;
    let first = chain.first().copied().unwrap_or(0);
    put(vol, slot, &dir_entry(name, ATTR_FILE, first, data.len() as u32));
    Ok(())
}

/// Crea un file nella radice.
pub fn create_file(vol: &mut [u8], name: &[u8; 11], data: &[u8]) -> Result<(), FatError> {
    create_file_in(vol, ROOT, name, data)
}

/// Crea una sottodirectory `name` dentro `parent`. Restituisce il suo cluster.
pub fn create_dir(vol: &mut [u8], parent: u32, name: &[u8; 11]) -> Result<u32, FatError> {
    let g = Geo::of(vol)?;
    let slot = g.slot(vol, parent, name)?;
    let c = g.alloc(vol, 1)?[0];
    // ".." di una sottodirectory della radice punta a 0, per convenzione FAT.
    let up = if parent == ROOT { 0 } else { parent };
    let mut dots = [0u8; 64];
    dots[0..32].copy_from_slice(&dir_entry(b".          ", ATTR_DIR, c, 0));
    dots[32..64].copy_from_slice(&dir_entry(b"..         ", ATTR_DIR, up, 0));
    g.write_chain(vol, &[c], &dots)?;
    put(vol, slot, &dir_entry(name, ATTR_DIR, c, 0));
    Ok(c)
}

/// Esito di `check`: cluster allocati ma non raggiungibili, e cluster condivisi.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    pub lost: Vec<u32>,
    pub cross_linked: Vec<u32>,
}

impl Report {
    pub fn is_clean(&self) -> bool {
        self.lost.is_empty() && self.cross_linked.is_empty()
    }
}

/// Percorre la radice e tutte le sottodirectory. Confronta i cluster raggiungibili con la FAT.
pub fn check(vol: &[u8]) -> Result<Report, FatError> {
    let g = Geo::of(vol)?;
    let fs = Fat32::open(vol)?;
    let mut reach = vec![false; g.clusters as usize + 2];
    let mut rep = Report::default();
    let mut todo: Vec<(u32, bool)> = vec![(ROOT, true)];
    while let Some((first, is_dir)) = todo.pop() {
        let chain = fs.chain(first)?;
        let mut dup = false;
        for c in chain {
            if reach[c as usize] {
                rep.cross_linked.push(c);
                dup = true;
            } else {
                reach[c as usize] = true;
            }
        }
        // Un cluster già visto ferma la discesa: evita cicli tra directory.
        if dup || !is_dir {
            continue;
        }
        for DirEntry { first_cluster, is_dir, .. } in fs.read_dir(first)? {
            if first_cluster >= 2 {
                todo.push((first_cluster, is_dir));
            }
        }
    }
    for c in 2..g.clusters + 2 {
        if g.get(vol, c) != 0 && !reach[c as usize] {
            rep.lost.push(c);
        }
    }
    Ok(rep)
}

/// Libera i cluster persi (dopo crash). Rifiuta se ci sono cluster condivisi: lì non si può decidere da soli.
pub fn reclaim_lost(vol: &mut [u8]) -> Result<usize, FatError> {
    let rep = check(vol)?;
    if !rep.cross_linked.is_empty() {
        return Err(FatError::CrossLinked);
    }
    let g = Geo::of(vol)?;
    for &c in &rep.lost {
        g.set(vol, c, 0);
    }
    Ok(rep.lost.len())
}
