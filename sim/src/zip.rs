//! Lettore ZIP in sola lettura, senza allocazione (no_std). Metodi: 0 (stored) e 8 (DEFLATE).
//! Niente ZIP64, niente cifratura. Il CRC-32 di ogni voce estratta viene verificato.

use crate::inflate::inflate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    BadData,
    Truncated,
    Full,
    Unsupported,
    Crc,
    NotFound,
}

const SIG_EOCD: u32 = 0x0605_4b50;
const SIG_CD: u32 = 0x0201_4b50;
const SIG_LOCAL: u32 = 0x0403_4b50;

fn u16_at(d: &[u8], i: usize) -> Result<u16, Error> {
    let s = d.get(i..i + 2).ok_or(Error::Truncated)?;
    Ok(u16::from_le_bytes([s[0], s[1]]))
}

fn u32_at(d: &[u8], i: usize) -> Result<u32, Error> {
    let s = d.get(i..i + 4).ok_or(Error::Truncated)?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// CRC-32 (IEEE, bit a bit: niente tabella, poca memoria).
pub fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 };
        }
    }
    !c
}

/// Una voce dell'archivio, dall'intestazione centrale.
#[derive(Debug, Clone, Copy)]
pub struct Entry<'a> {
    pub name: &'a [u8],
    pub method: u16,
    pub csize: u32,
    pub usize_: u32,
    pub crc: u32,
    pub offset: u32,
}

pub struct Archive<'a> {
    data: &'a [u8],
    cd: usize,
    count: usize,
}

impl<'a> Archive<'a> {
    /// Cerca la fine dell'archivio (EOCD), negli ultimi 64 KiB + 22 byte.
    pub fn open(data: &'a [u8]) -> Result<Archive<'a>, Error> {
        let min = data.len().saturating_sub(65_557);
        let mut i = data.len().checked_sub(22).ok_or(Error::BadData)?;
        loop {
            if u32_at(data, i)? == SIG_EOCD {
                let count = u16_at(data, i + 10)? as usize;
                let cd = u32_at(data, i + 16)? as usize;
                if count == 0xffff || cd == 0xffff_ffff as usize {
                    return Err(Error::Unsupported);
                }
                return Ok(Archive { data, cd, count });
            }
            if i == min {
                return Err(Error::BadData);
            }
            i -= 1;
        }
    }

    /// Voci dell'archivio, in ordine.
    pub fn entries(&self) -> Entries<'a> {
        Entries { data: self.data, pos: self.cd, left: self.count }
    }

    pub fn find(&self, name: &str) -> Result<Entry<'a>, Error> {
        self.entries()
            .flatten()
            .find(|e| e.name == name.as_bytes())
            .ok_or(Error::NotFound)
    }

    /// Estrae una voce in `out`; restituisce i byte scritti e verifica il CRC.
    pub fn extract(&self, e: &Entry, out: &mut [u8]) -> Result<usize, Error> {
        let lh = e.offset as usize;
        if u32_at(self.data, lh)? != SIG_LOCAL {
            return Err(Error::BadData);
        }
        let start = lh + 30 + u16_at(self.data, lh + 26)? as usize + u16_at(self.data, lh + 28)? as usize;
        let src = self.data.get(start..start + e.csize as usize).ok_or(Error::Truncated)?;
        let n = match e.method {
            0 => {
                let dst = out.get_mut(..src.len()).ok_or(Error::Full)?;
                dst.copy_from_slice(src);
                src.len()
            }
            8 => inflate(src, out)?,
            _ => return Err(Error::Unsupported),
        };
        if n != e.usize_ as usize {
            return Err(Error::BadData);
        }
        if crc32(&out[..n]) != e.crc {
            return Err(Error::Crc);
        }
        Ok(n)
    }
}

pub struct Entries<'a> {
    data: &'a [u8],
    pos: usize,
    left: usize,
}

impl<'a> Iterator for Entries<'a> {
    type Item = Result<Entry<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        let d = self.data;
        let p = self.pos;
        let r = (|| {
            if u32_at(d, p)? != SIG_CD {
                return Err(Error::BadData);
            }
            let nlen = u16_at(d, p + 28)? as usize;
            let xlen = u16_at(d, p + 30)? as usize;
            let clen = u16_at(d, p + 32)? as usize;
            let name = d.get(p + 46..p + 46 + nlen).ok_or(Error::Truncated)?;
            Ok((
                Entry {
                    name,
                    method: u16_at(d, p + 10)?,
                    crc: u32_at(d, p + 16)?,
                    csize: u32_at(d, p + 20)?,
                    usize_: u32_at(d, p + 24)?,
                    offset: u32_at(d, p + 42)?,
                },
                46 + nlen + xlen + clen,
            ))
        })();
        match r {
            Ok((e, step)) => {
                self.pos += step;
                Some(Ok(e))
            }
            Err(x) => {
                self.left = 0;
                Some(Err(x))
            }
        }
    }
}
