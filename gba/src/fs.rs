//! File system in RAM: store a blocchi fissi, percorsi e contenuti separati,
//! checksum per blocco, scritture atomiche, directory (percorsi "a/b"), permessi per owner, quota.
//! Solo `core`: compila nel binario no_std e nei test host (`rustc --test`).
#![allow(dead_code)]

pub const BLOCK: usize = 64;
pub const BLOCKS: usize = 128;
pub const FILES: usize = 16;
pub const NAME_MAX: usize = 24;
pub const MAX_FILE: usize = BLOCK * BLOCKS;
const NONE: u8 = 0xFF;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FsError {
    NotFound,
    Exists,
    NoSpace,
    Denied,
    TooLarge,
    Corrupt,
    BadName,
    IsDir,
    NotEmpty,
    NoParent,
}

#[derive(Clone, Copy)]
struct Entry {
    used: bool,
    dir: bool,
    name: [u8; NAME_MAX],
    name_len: u8,
    owner: u8,
    head: u8,
    len: u16,
}

const EMPTY: Entry = Entry { used: false, dir: false, name: [0; NAME_MAX], name_len: 0, owner: 0, head: NONE, len: 0 };

fn fnv_with(mut h: u32, b: &[u8]) -> u32 {
    for &x in b {
        h ^= x as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

fn fnv(b: &[u8]) -> u32 {
    fnv_with(0x811c_9dc5, b)
}

/// Checksum del puntatore `next` del blocco `i`: lega il valore alla posizione, così un puntatore spostato è rilevato.
fn link_sum(i: usize, next: u8) -> u32 {
    fnv_with(0x811c_9dc5, &[i as u8, next])
}

/// Checksum dei metadati di una voce: nome, flag, owner, testa, lunghezza. Da verificare su ARM7.
fn meta_sum(e: &Entry) -> u32 {
    let h = fnv_with(0x811c_9dc5, &e.name[..e.name_len as usize]);
    fnv_with(h, &[e.used as u8, e.dir as u8, e.owner, e.head, e.name_len, e.len as u8, (e.len >> 8) as u8])
}

/// Percorso valido: niente componenti vuoti, niente '/' iniziale o finale.
fn valid(name: &[u8]) -> bool {
    !name.is_empty()
        && name.len() <= NAME_MAX
        && name[0] != b'/'
        && name[name.len() - 1] != b'/'
        && !name.windows(2).any(|w| w == b"//")
}

/// Directory che contiene il percorso. `None` = radice. "a/b" -> Some("a"), "a" -> None.
pub fn parent_of(name: &[u8]) -> Option<&[u8]> {
    name.iter().rposition(|&c| c == b'/').map(|i| &name[..i])
}

pub struct Fs {
    data: [[u8; BLOCK]; BLOCKS],
    sum: [u32; BLOCKS],
    next: [u8; BLOCKS],
    used: [bool; BLOCKS],
    files: [Entry; FILES],
    /// Prossimo blocco da provare (next-fit): le scritture girano su tutto lo store.
    cursor: usize,
    /// Checksum dei metadati per ogni voce (vedi `meta_sum`).
    msum: [u32; FILES],
    /// Checksum dei puntatori `next` per blocco (vedi `link_sum`).
    nsum: [u32; BLOCKS],
}

impl Fs {
    pub const fn new() -> Self {
        Self {
            data: [[0; BLOCK]; BLOCKS],
            sum: [0; BLOCKS],
            next: [NONE; BLOCKS],
            used: [false; BLOCKS],
            files: [EMPTY; FILES],
            cursor: 0,
            msum: [0; FILES],
            nsum: [0; BLOCKS],
        }
    }

    /// Vero se la catena che parte da `b` ha tutti i puntatori validi e non è ciclica.
    fn chain_ok(&self, mut b: u8) -> bool {
        let mut steps = 0;
        while b != NONE {
            let i = b as usize;
            if i >= BLOCKS || steps >= BLOCKS || self.nsum[i] != link_sum(i, self.next[i]) {
                return false;
            }
            b = self.next[i];
            steps += 1;
        }
        true
    }

    fn find(&self, name: &[u8]) -> Option<usize> {
        (0..FILES).find(|&i| {
            let e = &self.files[i];
            e.used && &e.name[..e.name_len as usize] == name
        })
    }

    fn entry(&self, id: usize) -> Result<Entry, FsError> {
        match self.files.get(id) {
            Some(e) if e.used && meta_sum(e) == self.msum[id] => Ok(*e),
            Some(e) if e.used => Err(FsError::Corrupt),
            _ => Err(FsError::NotFound),
        }
    }

    fn add(&mut self, name: &[u8], owner: u8, dir: bool) -> Result<usize, FsError> {
        if !valid(name) {
            return Err(FsError::BadName);
        }
        if self.find(name).is_some() {
            return Err(FsError::Exists);
        }
        if let Some(p) = parent_of(name) {
            let pid = self.find(p).ok_or(FsError::NoParent)?;
            if !self.files[pid].dir {
                return Err(FsError::NoParent);
            }
        }
        let id = (0..FILES).find(|&i| !self.files[i].used).ok_or(FsError::NoSpace)?;
        let mut e = EMPTY;
        e.used = true;
        e.dir = dir;
        e.owner = owner;
        e.name_len = name.len() as u8;
        e.name[..name.len()].copy_from_slice(name);
        self.files[id] = e;
        self.msum[id] = meta_sum(&e);
        Ok(id)
    }

    pub fn create(&mut self, name: &[u8], owner: u8) -> Result<usize, FsError> {
        self.add(name, owner, false)
    }

    pub fn mkdir(&mut self, name: &[u8], owner: u8) -> Result<usize, FsError> {
        self.add(name, owner, true)
    }

    pub fn find_name(&self, name: &[u8]) -> Option<usize> {
        self.find(name)
    }

    /// Nome e owner del file `id`, se esiste.
    pub fn info(&self, id: usize) -> Option<(&[u8], u8)> {
        self.files.get(id).filter(|e| e.used).map(|e| (&e.name[..e.name_len as usize], e.owner))
    }

    pub fn is_dir(&self, id: usize) -> bool {
        self.files.get(id).map_or(false, |e| e.used && e.dir)
    }

    pub fn len_of(&self, id: usize) -> Option<usize> {
        self.files.get(id).filter(|e| e.used).map(|e| e.len as usize)
    }

    /// Elementi diretti di `dir` (stringa vuota = radice). Ritorna quanti id ha scritto in `out`.
    pub fn list(&self, dir: &[u8], out: &mut [usize]) -> usize {
        let d = if dir.is_empty() { None } else { Some(dir) };
        let mut n = 0;
        for id in 0..FILES {
            let e = &self.files[id];
            if e.used && parent_of(&e.name[..e.name_len as usize]) == d && n < out.len() {
                out[n] = id;
                n += 1;
            }
        }
        n
    }

    /// Scrittura atomica: blocchi nuovi e dati prima, cambio di testa poi, liberazione della catena vecchia.
    /// Se fallisce prima del cambio di testa, il file resta com'era.
    pub fn write(&mut self, id: usize, caller: u8, grant: usize, bytes: &[u8]) -> Result<(), FsError> {
        let e = self.entry(id)?;
        if !self.chain_ok(e.head) {
            return Err(FsError::Corrupt);
        }
        if e.dir {
            return Err(FsError::IsDir);
        }
        if e.owner != caller {
            return Err(FsError::Denied);
        }
        if bytes.len() > grant || bytes.len() > MAX_FILE {
            return Err(FsError::TooLarge);
        }
        let need = bytes.len().div_ceil(BLOCK);
        let mut staged = [NONE; BLOCKS];
        let mut n = 0;
        for k in 0..BLOCKS {
            if n == need {
                break;
            }
            let b = (self.cursor + k) % BLOCKS;
            if !self.used[b] {
                staged[n] = b as u8;
                n += 1;
            }
        }
        if need > 0 && n == need {
            self.cursor = (staged[need - 1] as usize + 1) % BLOCKS;
        }
        if n < need {
            return Err(FsError::NoSpace);
        }
        for i in 0..need {
            let b = staged[i] as usize;
            let start = i * BLOCK;
            let end = (start + BLOCK).min(bytes.len());
            self.data[b] = [0; BLOCK];
            self.data[b][..end - start].copy_from_slice(&bytes[start..end]);
            self.sum[b] = fnv(&self.data[b]);
            self.next[b] = if i + 1 < need { staged[i + 1] } else { NONE };
            self.nsum[b] = link_sum(b, self.next[b]);
            self.used[b] = true;
        }
        let f = &mut self.files[id];
        let old = f.head;
        f.head = if need > 0 { staged[0] } else { NONE };
        f.len = bytes.len() as u16;
        let snap = *f;
        self.msum[id] = meta_sum(&snap);
        self.free_chain(old);
        Ok(())
    }

    /// Legge il file. Ogni blocco viene verificato con il checksum.
    pub fn read(&self, id: usize, caller: u8, out: &mut [u8]) -> Result<usize, FsError> {
        let e = self.entry(id)?;
        if e.dir {
            return Err(FsError::IsDir);
        }
        if e.owner != caller {
            return Err(FsError::Denied);
        }
        let len = e.len as usize;
        let mut b = e.head;
        let mut pos = 0;
        while b != NONE && pos < len {
            let i = b as usize;
            if i >= BLOCKS || fnv(&self.data[i]) != self.sum[i] || self.nsum[i] != link_sum(i, self.next[i]) {
                return Err(FsError::Corrupt);
            }
            let take = (len - pos).min(BLOCK);
            for k in 0..take {
                if pos + k < out.len() {
                    out[pos + k] = self.data[i][k];
                }
            }
            pos += take;
            b = self.next[i];
        }
        Ok(len.min(out.len()))
    }

    /// Rimuove un file o una directory vuota.
    pub fn remove(&mut self, id: usize, caller: u8) -> Result<(), FsError> {
        let e = self.entry(id)?;
        if !self.chain_ok(e.head) {
            return Err(FsError::Corrupt);
        }
        if e.owner != caller {
            return Err(FsError::Denied);
        }
        if e.dir {
            let path = &e.name[..e.name_len as usize];
            let has_child = (0..FILES).any(|i| {
                let c = &self.files[i];
                c.used && parent_of(&c.name[..c.name_len as usize]) == Some(path)
            });
            if has_child {
                return Err(FsError::NotEmpty);
            }
        }
        self.free_chain(e.head);
        self.files[id] = EMPTY;
        Ok(())
    }

    fn free_chain(&mut self, mut b: u8) {
        while b != NONE {
            let i = b as usize;
            self.used[i] = false;
            b = self.next[i];
        }
    }

    pub fn free_blocks(&self) -> usize {
        self.used.iter().filter(|&&u| !u).count()
    }

    #[cfg(test)]
    fn tamper_next(&mut self, id: usize) {
        let b = self.files[id].head as usize;
        self.next[b] ^= 1;
    }

    #[cfg(test)]
    fn tamper_meta(&mut self, id: usize) {
        self.files[id].owner ^= 1;
    }

    #[cfg(test)]
    fn corrupt(&mut self, id: usize) {
        let b = self.files[id].head as usize;
        self.data[b][0] ^= 0xFF;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_blocks() {
        let mut fs = Fs::new();
        let id = fs.create(b"a.txt", 1).unwrap();
        let msg = [7u8; 200];
        fs.write(id, 1, 1024, &msg).unwrap();
        assert_eq!(fs.free_blocks(), BLOCKS - 4);
        let mut out = [0u8; 200];
        assert_eq!(fs.read(id, 1, &mut out), Ok(200));
        assert_eq!(out, msg);
    }

    #[test]
    fn owner_and_quota() {
        let mut fs = Fs::new();
        let id = fs.create(b"x", 1).unwrap();
        assert_eq!(fs.write(id, 2, 1024, b"hi"), Err(FsError::Denied));
        assert_eq!(fs.write(id, 1, 4, b"hello!"), Err(FsError::TooLarge));
        assert_eq!(fs.create(b"x", 1), Err(FsError::Exists));
        assert_eq!(fs.create(b"", 1), Err(FsError::BadName));
    }

    #[test]
    fn rewrite_frees_old_chain() {
        let mut fs = Fs::new();
        let id = fs.create(b"r", 0).unwrap();
        fs.write(id, 0, 1024, &[1u8; 300]).unwrap();
        fs.write(id, 0, 1024, b"ok").unwrap();
        assert_eq!(fs.free_blocks(), BLOCKS - 1);
        let mut out = [0u8; 2];
        assert_eq!(fs.read(id, 0, &mut out), Ok(2));
        assert_eq!(&out, b"ok");
    }

    #[test]
    fn atomic_when_no_space() {
        let mut fs = Fs::new();
        let a = fs.create(b"a", 0).unwrap();
        fs.write(a, 0, 1 << 20, b"keep me").unwrap();
        let big = fs.create(b"big", 0).unwrap();
        let filler = [0u8; MAX_FILE - BLOCK];
        fs.write(big, 0, 1 << 20, &filler).unwrap();
        assert_eq!(fs.write(a, 0, 1 << 20, &[9u8; MAX_FILE]), Err(FsError::NoSpace));
        let mut out = [0u8; 7];
        assert_eq!(fs.read(a, 0, &mut out), Ok(7));
        assert_eq!(&out, b"keep me");
    }

    #[test]
    fn writes_spread_over_blocks() {
        let mut fs = Fs::new();
        let id = fs.create(b"w", 0).unwrap();
        let mut heads = [0u8; 4];
        for h in heads.iter_mut() {
            fs.write(id, 0, 1024, &[1u8; 10]).unwrap();
            *h = fs.files[id].head;
        }
        assert!(heads.windows(2).all(|p| p[0] != p[1]));
    }

    #[test]
    fn metadata_corruption_detected() {
        let mut fs = Fs::new();
        let id = fs.create(b"m", 0).unwrap();
        fs.write(id, 0, 1024, b"data").unwrap();
        fs.tamper_meta(id);
        let mut out = [0u8; 4];
        assert_eq!(fs.read(id, 0, &mut out), Err(FsError::Corrupt));
        assert_eq!(fs.remove(id, 0), Err(FsError::Corrupt));
    }

    #[test]
    fn next_pointer_corruption_detected() {
        let mut fs = Fs::new();
        let id = fs.create(b"p", 0).unwrap();
        fs.write(id, 0, 1024, &[7u8; 100]).unwrap();
        fs.tamper_next(id);
        let mut out = [0u8; 100];
        assert_eq!(fs.read(id, 0, &mut out), Err(FsError::Corrupt));
        assert_eq!(fs.write(id, 0, 1024, b"new"), Err(FsError::Corrupt));
        assert_eq!(fs.remove(id, 0), Err(FsError::Corrupt));
    }

    #[test]
    fn corruption_detected_and_remove() {
        let mut fs = Fs::new();
        let id = fs.create(b"c", 0).unwrap();
        fs.write(id, 0, 1024, b"data").unwrap();
        fs.corrupt(id);
        let mut out = [0u8; 4];
        assert_eq!(fs.read(id, 0, &mut out), Err(FsError::Corrupt));
        fs.remove(id, 0).unwrap();
        assert_eq!(fs.free_blocks(), BLOCKS);
        assert_eq!(fs.read(id, 0, &mut out), Err(FsError::NotFound));
    }

    #[test]
    fn directories_and_children() {
        let mut fs = Fs::new();
        let d = fs.mkdir(b"docs", 0).unwrap();
        assert_eq!(fs.create(b"nope/f", 0), Err(FsError::NoParent));
        let f = fs.create(b"docs/a", 0).unwrap();
        fs.write(f, 0, 1024, b"x").unwrap();
        assert_eq!(fs.write(d, 0, 1024, b"x"), Err(FsError::IsDir));
        assert_eq!(fs.remove(d, 0), Err(FsError::NotEmpty));
        let mut ids = [0usize; FILES];
        assert_eq!(fs.list(b"", &mut ids), 1);
        assert_eq!(ids[0], d);
        assert_eq!(fs.list(b"docs", &mut ids), 1);
        assert_eq!(ids[0], f);
        fs.remove(f, 0).unwrap();
        fs.remove(d, 0).unwrap();
    }
}
