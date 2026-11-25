//! File system in RAM: store a blocchi fissi, nomi e contenuti separati,
//! checksum per blocco, scritture atomiche, permessi legati all'owner e alla quota.
//! Solo `core`: compila nel binario no_std e nei test host (`rustc --test`).
#![allow(dead_code)]

pub const BLOCK: usize = 64;
pub const BLOCKS: usize = 128;
pub const FILES: usize = 16;
pub const NAME_MAX: usize = 12;
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
}

#[derive(Clone, Copy)]
struct Entry {
    used: bool,
    name: [u8; NAME_MAX],
    name_len: u8,
    owner: u8,
    head: u8,
    len: u16,
}

const EMPTY: Entry = Entry { used: false, name: [0; NAME_MAX], name_len: 0, owner: 0, head: NONE, len: 0 };

fn fnv(b: &[u8]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for &x in b {
        h ^= x as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

pub struct Fs {
    data: [[u8; BLOCK]; BLOCKS],
    sum: [u32; BLOCKS],
    next: [u8; BLOCKS],
    used: [bool; BLOCKS],
    files: [Entry; FILES],
}

impl Fs {
    pub const fn new() -> Self {
        Self {
            data: [[0; BLOCK]; BLOCKS],
            sum: [0; BLOCKS],
            next: [NONE; BLOCKS],
            used: [false; BLOCKS],
            files: [EMPTY; FILES],
        }
    }

    fn find(&self, name: &[u8]) -> Option<usize> {
        (0..FILES).find(|&i| {
            let e = &self.files[i];
            e.used && &e.name[..e.name_len as usize] == name
        })
    }

    fn entry(&self, id: usize) -> Result<Entry, FsError> {
        match self.files.get(id) {
            Some(e) if e.used => Ok(*e),
            _ => Err(FsError::NotFound),
        }
    }

    pub fn create(&mut self, name: &[u8], owner: u8) -> Result<usize, FsError> {
        if name.is_empty() || name.len() > NAME_MAX {
            return Err(FsError::BadName);
        }
        if self.find(name).is_some() {
            return Err(FsError::Exists);
        }
        let id = (0..FILES).find(|&i| !self.files[i].used).ok_or(FsError::NoSpace)?;
        let mut e = EMPTY;
        e.used = true;
        e.owner = owner;
        e.name_len = name.len() as u8;
        e.name[..name.len()].copy_from_slice(name);
        self.files[id] = e;
        Ok(id)
    }

    /// Scrittura atomica: blocchi nuovi e dati prima, cambio di testa poi, liberazione della catena vecchia.
    /// Se fallisce prima del cambio di testa, il file resta com'era.
    pub fn write(&mut self, id: usize, caller: u8, grant: usize, bytes: &[u8]) -> Result<(), FsError> {
        let e = self.entry(id)?;
        if e.owner != caller {
            return Err(FsError::Denied);
        }
        if bytes.len() > grant || bytes.len() > MAX_FILE {
            return Err(FsError::TooLarge);
        }
        let need = bytes.len().div_ceil(BLOCK);
        let mut staged = [NONE; BLOCKS];
        let mut n = 0;
        for b in 0..BLOCKS {
            if n == need {
                break;
            }
            if !self.used[b] {
                staged[n] = b as u8;
                n += 1;
            }
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
            self.used[b] = true;
        }
        let f = &mut self.files[id];
        let old = f.head;
        f.head = if need > 0 { staged[0] } else { NONE };
        f.len = bytes.len() as u16;
        self.free_chain(old);
        Ok(())
    }

    /// Legge il file. Ogni blocco viene verificato con il checksum.
    pub fn read(&self, id: usize, caller: u8, out: &mut [u8]) -> Result<usize, FsError> {
        let e = self.entry(id)?;
        if e.owner != caller {
            return Err(FsError::Denied);
        }
        let len = e.len as usize;
        let mut b = e.head;
        let mut pos = 0;
        while b != NONE && pos < len {
            let i = b as usize;
            if fnv(&self.data[i]) != self.sum[i] {
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

    pub fn remove(&mut self, id: usize, caller: u8) -> Result<(), FsError> {
        let e = self.entry(id)?;
        if e.owner != caller {
            return Err(FsError::Denied);
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
}
