//! Lettore exFAT in sola lettura su un volume in memoria. Nessuna scrittura, nessun bitmap.
//! Specifica: Microsoft exFAT (Open Specifications). Il sandbox non l'ha raggiunta: gli offset vengono
//! dall'header Linux fs/exfat/exfat_raw.h, il checksum da fs/exfat/dir.c. **Da verificare** sul testo ufficiale.
//! Vive nel programma disco, non nel nucleo.

const BAD: u32 = 0xFFFF_FFF7;
const ET_END: u8 = 0x00;
const ET_FILE: u8 = 0x85;
const ET_STREAM: u8 = 0xC0;
const ET_NAME: u8 = 0xC1;
const ATTR_DIR: u16 = 0x10;
const FLAG_NO_CHAIN: u8 = 0x02;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExFatError {
    TooSmall,
    NoSignature,
    NotExFat,
    BadBpb,
    BadCluster,
    Loop,
    OutOfRange,
    IsDir,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub first_cluster: u32,
    pub size: u64,
    /// Cluster contigui: niente catena nella FAT.
    pub contiguous: bool,
}

pub struct ExFat<'a> {
    vol: &'a [u8],
    bps: usize,
    spc: usize,
    fat_off: usize,
    heap_off: usize,
    cluster_count: u32,
    root: u32,
}

fn u16_at(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn u64_at(d: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(d[o..o + 8].try_into().unwrap())
}

/// Checksum di un insieme di voci (lo slot 2-3 della prima voce è escluso).
pub fn set_checksum(set: &[u8]) -> u16 {
    let mut s: u16 = 0;
    for (i, &b) in set.iter().enumerate() {
        if i == 2 || i == 3 {
            continue;
        }
        s = ((s << 15) | (s >> 1)).wrapping_add(b as u16);
    }
    s
}

impl<'a> ExFat<'a> {
    pub fn open(vol: &'a [u8]) -> Result<Self, ExFatError> {
        if vol.len() < 512 {
            return Err(ExFatError::TooSmall);
        }
        if vol[510] != 0x55 || vol[511] != 0xAA {
            return Err(ExFatError::NoSignature);
        }
        if &vol[3..11] != b"EXFAT   " || vol[11..64].iter().any(|&b| b != 0) {
            return Err(ExFatError::NotExFat);
        }
        let bits = vol[108];
        let spc_bits = vol[109];
        if !(9..=12).contains(&bits) || spc_bits as u32 > 25 - bits as u32 || vol[110] != 1 {
            return Err(ExFatError::BadBpb); // un solo FAT: TexFAT non supportato
        }
        let bps = 1usize << bits;
        let spc = 1usize << spc_bits;
        let fat_off = u32_at(vol, 80) as usize;
        let fat_len = u32_at(vol, 84) as usize;
        let heap_off = u32_at(vol, 88) as usize;
        let cluster_count = u32_at(vol, 92);
        let root = u32_at(vol, 96);
        if cluster_count > 0xFFFF_FFF5 || root < 2 || root > cluster_count + 1 {
            return Err(ExFatError::BadBpb);
        }
        // FAT e area dati devono stare nel volume (offset in settori).
        let total = vol.len() / bps;
        if (cluster_count as usize + 2) * 4 > fat_len * bps
            || fat_off * bps + fat_len * bps > vol.len()
            || heap_off + cluster_count as usize * spc > total
        {
            return Err(ExFatError::OutOfRange);
        }
        Ok(ExFat { vol, bps, spc, fat_off: fat_off * bps, heap_off: heap_off * bps, cluster_count, root })
    }

    fn cluster_off(&self, c: u32) -> Result<usize, ExFatError> {
        if c < 2 || c - 2 >= self.cluster_count {
            return Err(ExFatError::BadCluster);
        }
        Ok(self.heap_off + (c - 2) as usize * self.spc * self.bps)
    }

    pub fn next(&self, c: u32) -> Result<u32, ExFatError> {
        if c < 2 || c > self.cluster_count + 1 {
            return Err(ExFatError::BadCluster);
        }
        Ok(u32_at(self.vol, self.fat_off + c as usize * 4))
    }

    /// Clusters di un oggetto: catena nella FAT, oppure contigui se il flag NoFatChain è impostato.
    fn clusters(&self, first: u32, size: u64, contiguous: bool) -> Result<Vec<u32>, ExFatError> {
        let cb = (self.spc * self.bps) as u64;
        if contiguous {
            let n = size.div_ceil(cb).max(1) as u32;
            let last = first.checked_add(n - 1).ok_or(ExFatError::BadCluster)?;
            self.cluster_off(first)?;
            self.cluster_off(last)?;
            return Ok((first..=last).collect());
        }
        let mut out = Vec::new();
        let mut c = first;
        loop {
            self.cluster_off(c)?;
            out.push(c);
            if out.len() > self.cluster_count as usize {
                return Err(ExFatError::Loop);
            }
            let n = self.next(c)?;
            if n >= 0xFFFF_FFF8 {
                return Ok(out);
            }
            if n == BAD || n < 2 {
                return Err(ExFatError::BadCluster);
            }
            c = n;
        }
    }

    pub fn root_dir(&self) -> Result<Vec<DirEntry>, ExFatError> {
        self.read_dir(self.root, 0, false)
    }

    /// Voci di una directory. Un insieme con checksum errato viene saltato.
    pub fn read_dir(&self, first: u32, size: u64, contiguous: bool) -> Result<Vec<DirEntry>, ExFatError> {
        let cb = self.spc * self.bps;
        let mut raw: Vec<[u8; 32]> = Vec::new();
        'outer: for c in self.clusters(first, size, contiguous)? {
            let base = self.cluster_off(c)?;
            for e in self.vol[base..base + cb].chunks_exact(32) {
                if e[0] == ET_END {
                    break 'outer;
                }
                raw.push(e.try_into().unwrap());
            }
        }
        let mut out = Vec::new();
        let mut i = 0;
        while i < raw.len() {
            let e = raw[i];
            if e[0] != ET_FILE {
                i += 1; // voci cancellate, etichetta, bitmap, upcase: non servono
                continue;
            }
            let n = e[1] as usize;
            if n < 2 || i + n >= raw.len() {
                break;
            }
            let set: Vec<u8> = raw[i..=i + n].iter().flatten().copied().collect();
            let stored = u16_at(&e, 2);
            let s = raw[i + 1];
            if s[0] != ET_STREAM || set_checksum(&set) != stored {
                i += 1 + n;
                continue;
            }
            let name_len = s[3] as usize;
            let mut units: Vec<u16> = Vec::new();
            for k in 2..=n {
                let ne = raw[i + k];
                if ne[0] != ET_NAME {
                    break;
                }
                for j in 0..15 {
                    units.push(u16_at(&ne, 2 + 2 * j));
                }
            }
            units.truncate(name_len);
            let contiguous = s[1] & FLAG_NO_CHAIN != 0;
            out.push(DirEntry {
                name: String::from_utf16_lossy(&units),
                is_dir: u16_at(&e, 4) & ATTR_DIR != 0,
                first_cluster: u32_at(&s, 20),
                size: u64_at(&s, 24),
                contiguous,
            });
            i += 1 + n;
        }
        Ok(out)
    }

    pub fn read_file(&self, f: &DirEntry) -> Result<Vec<u8>, ExFatError> {
        if f.is_dir {
            return Err(ExFatError::IsDir);
        }
        let size = f.size as usize;
        if size == 0 {
            return Ok(Vec::new());
        }
        let cb = self.spc * self.bps;
        let mut out = Vec::with_capacity(size);
        for c in self.clusters(f.first_cluster, f.size, f.contiguous)? {
            let base = self.cluster_off(c)?;
            out.extend_from_slice(&self.vol[base..base + cb]);
            if out.len() >= size {
                break;
            }
        }
        if out.len() < size {
            return Err(ExFatError::OutOfRange);
        }
        out.truncate(size);
        Ok(out)
    }
}
