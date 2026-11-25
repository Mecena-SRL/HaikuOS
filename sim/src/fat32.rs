//! Lettore FAT32 in sola lettura su un volume in memoria. Nessuna scrittura.
//! Specifica: Microsoft "FAT specification" (fatgen103). Il sandbox non ha raggiunto il testo ufficiale:
//! offset e regole sono verificati solo sull'implementazione Linux (fs/fat, GPL). **Da verificare**.
//! Vive nel programma disco, non nel nucleo.

pub const FAT32_MIN_CLUSTERS: u32 = 65525;
const EOC_MIN: u32 = 0x0FFF_FFF8;
const BAD_CLUSTER: u32 = 0x0FFF_FFF7;
const ATTR_DIR: u8 = 0x10;
const ATTR_VOLUME: u8 = 0x08;
const ATTR_LFN: u8 = 0x0F;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FatError {
    TooSmall,
    NoSignature,
    BadBpb,
    /// Meno di 65525 cluster: il volume è FAT12/16 per definizione, non FAT32.
    NotFat32,
    BadCluster,
    /// Catena di cluster che torna su sé stessa.
    Loop,
    OutOfRange,
    IsDir,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub first_cluster: u32,
    pub size: u32,
}

pub struct Fat32<'a> {
    vol: &'a [u8],
    bps: usize,
    spc: usize,
    fat_start: usize,
    data_start: usize,
    cluster_count: u32,
}

fn u16_at(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

/// Checksum del nome corto, usato per legare le voci LFN al loro nome 8.3.
pub fn short_checksum(b: &[u8]) -> u8 {
    b.iter().fold(0u8, |s, &c| (s << 7).wrapping_add(s >> 1).wrapping_add(c))
}

fn short_name(b: &[u8]) -> String {
    let mut first = b[0];
    if first == 0x05 {
        first = 0xE5; // 0x05 in prima posizione rappresenta 0xE5
    }
    let base: String = std::iter::once(first).chain(b[1..8].iter().copied())
        .map(|c| c as char).collect::<String>().trim_end().to_string();
    let ext: String = b[8..11].iter().map(|&c| c as char).collect::<String>().trim_end().to_string();
    if ext.is_empty() { base } else { format!("{base}.{ext}") }
}

impl<'a> Fat32<'a> {
    /// Legge il boot sector e controlla i campi essenziali. Il tipo FAT32 si decide dal numero di cluster.
    pub fn open(vol: &'a [u8]) -> Result<Self, FatError> {
        if vol.len() < 512 {
            return Err(FatError::TooSmall);
        }
        if vol[510] != 0x55 || vol[511] != 0xAA {
            return Err(FatError::NoSignature);
        }
        let bps = u16_at(vol, 11) as usize;
        let spc = vol[13] as usize;
        let rsvd = u16_at(vol, 14) as usize;
        let nfats = vol[16] as usize;
        let bad = !matches!(bps, 512 | 1024 | 2048 | 4096)
            || spc == 0
            || !spc.is_power_of_two()
            || nfats == 0
            || rsvd == 0
            || u16_at(vol, 17) != 0 // root entries: zero in FAT32
            || u16_at(vol, 19) != 0 // total sectors 16: zero in FAT32
            || u16_at(vol, 22) != 0 // FATSz16: zero in FAT32
            || u32_at(vol, 36) == 0; // FATSz32
        if bad {
            return Err(FatError::BadBpb);
        }
        let fatsz = u32_at(vol, 36) as usize;
        let tot = u32_at(vol, 32) as usize;
        let root = u32_at(vol, 44);
        let overhead = rsvd + nfats * fatsz;
        let data_sectors = tot.checked_sub(overhead).ok_or(FatError::BadBpb)?;
        let cluster_count = (data_sectors / spc) as u32;
        if cluster_count < FAT32_MIN_CLUSTERS {
            return Err(FatError::NotFat32);
        }
        if root < 2 || root > cluster_count + 1 {
            return Err(FatError::BadBpb);
        }
        let fat_start = rsvd * bps;
        let data_start = overhead * bps;
        let fat_bytes = fatsz * bps;
        // La FAT deve coprire tutti i cluster; l'area dati deve stare nel volume.
        if (cluster_count as usize + 2) * 4 > fat_bytes
            || fat_start + fat_bytes > vol.len()
            || data_start + cluster_count as usize * spc * bps > vol.len()
        {
            return Err(FatError::OutOfRange);
        }
        Ok(Fat32 { vol, bps, spc, fat_start, data_start, cluster_count })
    }

    fn cluster_off(&self, c: u32) -> Result<usize, FatError> {
        if c < 2 || c - 2 >= self.cluster_count {
            return Err(FatError::BadCluster);
        }
        Ok(self.data_start + (c - 2) as usize * self.spc * self.bps)
    }

    /// Voce della FAT per il cluster `c` (28 bit utili).
    pub fn next(&self, c: u32) -> Result<u32, FatError> {
        if c < 2 || c > self.cluster_count + 1 {
            return Err(FatError::BadCluster);
        }
        Ok(u32_at(self.vol, self.fat_start + c as usize * 4) & 0x0FFF_FFFF)
    }

    /// Catena di cluster a partire da `start`. Si ferma alla fine (EOC) e rileva i cicli.
    pub fn chain(&self, start: u32) -> Result<Vec<u32>, FatError> {
        let mut out = Vec::new();
        let mut c = start;
        loop {
            self.cluster_off(c)?;
            out.push(c);
            if out.len() > self.cluster_count as usize {
                return Err(FatError::Loop);
            }
            let n = self.next(c)?;
            if n >= EOC_MIN {
                return Ok(out);
            }
            if n == BAD_CLUSTER || n < 2 {
                return Err(FatError::BadCluster);
            }
            c = n;
        }
    }

    /// Voci di una directory, senza "." e "..". Un nome LFN è usato solo se il checksum combacia.
    pub fn read_dir(&self, first: u32) -> Result<Vec<DirEntry>, FatError> {
        let mut out = Vec::new();
        let mut lfn: Vec<(u8, Vec<u16>)> = Vec::new();
        let mut lfn_sum: u8 = 0;
        let cluster_bytes = self.spc * self.bps;
        for c in self.chain(first)? {
            let base = self.cluster_off(c)?;
            for e in self.vol[base..base + cluster_bytes].chunks_exact(32) {
                if e[0] == 0x00 {
                    return Ok(out); // fine della directory
                }
                if e[0] == 0xE5 {
                    lfn.clear();
                    continue; // voce cancellata
                }
                if e[11] == ATTR_LFN {
                    if e[0] & 0x40 != 0 {
                        lfn.clear();
                        lfn_sum = e[13];
                    }
                    let offs = [1, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 28, 30];
                    let chars = offs.iter().map(|&o| u16::from_le_bytes([e[o], e[o + 1]])).collect();
                    lfn.push((e[0] & 0x1F, chars));
                    continue;
                }
                let use_lfn = !lfn.is_empty() && lfn_sum == short_checksum(&e[0..11]);
                if e[11] & ATTR_VOLUME != 0 {
                    lfn.clear();
                    continue; // etichetta del volume
                }
                let name = if use_lfn {
                    lfn.sort_by_key(|(seq, _)| *seq);
                    let units: Vec<u16> = lfn.iter().flat_map(|(_, c)| c.iter().copied())
                        .take_while(|&u| u != 0 && u != 0xFFFF).collect();
                    String::from_utf16_lossy(&units)
                } else {
                    short_name(&e[0..11])
                };
                lfn.clear();
                if name == "." || name == ".." {
                    continue;
                }
                let first_cluster = (u16_at(e, 20) as u32) << 16 | u16_at(e, 26) as u32;
                out.push(DirEntry {
                    name,
                    is_dir: e[11] & ATTR_DIR != 0,
                    first_cluster,
                    size: u32_at(e, 28),
                });
            }
        }
        Ok(out)
    }

    /// Contenuto di un file. Legge la catena e la tronca alla dimensione dichiarata.
    pub fn read_file(&self, f: &DirEntry) -> Result<Vec<u8>, FatError> {
        if f.is_dir {
            return Err(FatError::IsDir);
        }
        let size = f.size as usize;
        if size == 0 {
            return Ok(Vec::new());
        }
        let cluster_bytes = self.spc * self.bps;
        let mut out = Vec::with_capacity(size);
        for c in self.chain(f.first_cluster)? {
            let base = self.cluster_off(c)?;
            out.extend_from_slice(&self.vol[base..base + cluster_bytes]);
            if out.len() >= size {
                break;
            }
        }
        if out.len() < size {
            return Err(FatError::OutOfRange);
        }
        out.truncate(size);
        Ok(out)
    }
}
