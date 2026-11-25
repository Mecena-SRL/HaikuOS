//! Scrittura FAT32 nel simulatore, dietro il flag `fat32-write`. Solo radice piatta, nomi 8.3, cluster da 512 byte.
//! Ordine di scrittura: dati, FAT (entrambe le copie), voce di directory per ultima: è il punto di commit.
//! Stessa specifica del lettore (`fat32.rs`). **Da verificare** come per il lettore.

use crate::fat32::{Fat32, FatError, FAT32_MIN_CLUSTERS};

const BPS: usize = 512;
const RSVD: usize = 32;
const NFATS: usize = 2;
const EOC: u32 = 0x0FFF_FFFF;

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
    // Azzera riservati, FAT e radice.
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
    put(vol, 44, &2u32.to_le_bytes());
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

/// Crea un file nella radice. `name` è già in forma 8.3 (11 byte, spazi di riempimento).
pub fn create_file(vol: &mut [u8], name: &[u8; 11], data: &[u8]) -> Result<(), FatError> {
    let (bps, spc, fat_start, clusters) = {
        let fs = Fat32::open(vol)?;
        fs.geometry()
    };
    if bps != BPS || spc != 1 {
        return Err(FatError::BadBpb);
    }
    let fatsz = u32_at(vol, 36) as usize;
    let fat2 = fat_start + fatsz * BPS;
    let data_start = fat_start + NFATS * fatsz * BPS;

    // Radice: nome già presente? primo slot libero.
    let mut slot = None;
    for k in 0..BPS / 32 {
        let o = data_start + k * 32;
        let e = &vol[o..o + 32];
        if e[0] == 0x00 {
            slot.get_or_insert(o);
            break;
        }
        if e[0] == 0xE5 {
            slot.get_or_insert(o);
        } else if e[11] & 0x0F != 0x0F && e[11] & 0x08 == 0 && &e[0..11] == name {
            return Err(FatError::Exists);
        }
    }
    let slot = slot.ok_or(FatError::DirFull)?;

    // Cluster liberi (voce FAT = 0), dal cluster 3.
    let n = data.len().div_ceil(BPS);
    let mut free = Vec::with_capacity(n);
    for c in 3..clusters + 2 {
        if free.len() == n {
            break;
        }
        if u32_at(vol, fat_start + c as usize * 4) & 0x0FFF_FFFF == 0 {
            free.push(c);
        }
    }
    if free.len() < n {
        return Err(FatError::NoSpace);
    }

    // 1) dati
    for (i, &c) in free.iter().enumerate() {
        let off = data_start + (c as usize - 2) * BPS;
        let chunk = &data[i * BPS..((i + 1) * BPS).min(data.len())];
        vol[off..off + BPS].fill(0);
        put(vol, off, chunk);
    }
    // 2) FAT, entrambe le copie
    for (i, &c) in free.iter().enumerate() {
        let val = if i + 1 < n { free[i + 1] } else { EOC };
        for base in [fat_start, fat2] {
            put(vol, base + c as usize * 4, &val.to_le_bytes());
        }
    }
    // 3) voce di directory: commit
    let first = free.first().copied().unwrap_or(0);
    let mut e = [0u8; 32];
    e[0..11].copy_from_slice(name);
    e[11] = 0x20;
    e[20..22].copy_from_slice(&((first >> 16) as u16).to_le_bytes());
    e[26..28].copy_from_slice(&((first & 0xFFFF) as u16).to_le_bytes());
    e[28..32].copy_from_slice(&(data.len() as u32).to_le_bytes());
    put(vol, slot, &e);
    Ok(())
}
