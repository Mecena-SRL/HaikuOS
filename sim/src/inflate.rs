//! DEFLATE (RFC 1951) senza allocazione (no_std): legge `src`, scrive in `out`.
//! Decodifica Huffman bit a bit con tabelle canoniche: lenta ma corta. Base: come puff.c.

use crate::zip::Error;

const MAXLEN: usize = 288;

const LBASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEXT: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DBASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DEXT: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
const ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

/// Lettore di bit, LSB per primo (ordine DEFLATE).
struct Bits<'a> {
    src: &'a [u8],
    pos: usize,
    buf: u32,
    cnt: u32,
}

impl Bits<'_> {
    fn get(&mut self, n: u32) -> Result<u32, Error> {
        while self.cnt < n {
            let b = *self.src.get(self.pos).ok_or(Error::Truncated)?;
            self.pos += 1;
            self.buf |= (b as u32) << self.cnt;
            self.cnt += 8;
        }
        let v = self.buf & ((1u32 << n) - 1);
        self.buf >>= n;
        self.cnt -= n;
        Ok(v)
    }

    /// Scarta i bit residui del byte corrente (il buffer tiene meno di 8 bit).
    fn align(&mut self) {
        self.buf = 0;
        self.cnt = 0;
    }
}

/// Tabella Huffman canonica: conteggio per lunghezza e simboli ordinati.
struct Huff {
    count: [u16; 16],
    sym: [u16; MAXLEN],
}

impl Huff {
    fn build(lengths: &[u8]) -> Result<Huff, Error> {
        let mut h = Huff { count: [0; 16], sym: [0; MAXLEN] };
        for &l in lengths {
            h.count[l as usize] += 1;
        }
        // Codice sovrasaturo: più codici di quelli che la lunghezza permette.
        let mut left: i32 = 1;
        for len in 1..16 {
            left = (left << 1) - h.count[len] as i32;
            if left < 0 {
                return Err(Error::BadData);
            }
        }
        let mut offs = [0u16; 16];
        for len in 1..15 {
            offs[len + 1] = offs[len] + h.count[len];
        }
        for (s, &l) in lengths.iter().enumerate() {
            if l != 0 {
                h.sym[offs[l as usize] as usize] = s as u16;
                offs[l as usize] += 1;
            }
        }
        Ok(h)
    }

    fn decode(&self, b: &mut Bits) -> Result<u16, Error> {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..16 {
            code |= b.get(1)? as i32;
            let count = self.count[len] as i32;
            if code - count < first {
                return Ok(self.sym[(index + (code - first)) as usize]);
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err(Error::BadData)
    }
}

struct Out<'a> {
    buf: &'a mut [u8],
    n: usize,
}

impl Out<'_> {
    fn put(&mut self, b: u8) -> Result<(), Error> {
        *self.buf.get_mut(self.n).ok_or(Error::Full)? = b;
        self.n += 1;
        Ok(())
    }
}

/// Decodifica un flusso DEFLATE grezzo (senza intestazione zlib). Restituisce i byte scritti.
pub fn inflate(src: &[u8], out: &mut [u8]) -> Result<usize, Error> {
    let mut b = Bits { src, pos: 0, buf: 0, cnt: 0 };
    let mut o = Out { buf: out, n: 0 };
    loop {
        let last = b.get(1)?;
        match b.get(2)? {
            0 => stored(&mut b, &mut o)?,
            1 => {
                let mut l = [0u8; MAXLEN];
                for (i, v) in l.iter_mut().enumerate() {
                    *v = match i {
                        0..=143 => 8,
                        144..=255 => 9,
                        256..=279 => 7,
                        _ => 8,
                    };
                }
                let lit = Huff::build(&l)?;
                let dist = Huff::build(&[5u8; 30])?;
                codes(&mut b, &mut o, &lit, &dist)?;
            }
            2 => {
                let (lit, dist) = dynamic(&mut b)?;
                codes(&mut b, &mut o, &lit, &dist)?;
            }
            _ => return Err(Error::BadData),
        }
        if last == 1 {
            return Ok(o.n);
        }
    }
}

fn stored(b: &mut Bits, o: &mut Out) -> Result<(), Error> {
    b.align();
    let h = b.src.get(b.pos..b.pos + 4).ok_or(Error::Truncated)?;
    let len = u16::from_le_bytes([h[0], h[1]]) as usize;
    let nlen = u16::from_le_bytes([h[2], h[3]]) as usize;
    if len != !nlen & 0xffff {
        return Err(Error::BadData);
    }
    b.pos += 4;
    let data = b.src.get(b.pos..b.pos + len).ok_or(Error::Truncated)?;
    for &x in data {
        o.put(x)?;
    }
    b.pos += len;
    Ok(())
}

fn dynamic(b: &mut Bits) -> Result<(Huff, Huff), Error> {
    let nlen = b.get(5)? as usize + 257;
    let ndist = b.get(5)? as usize + 1;
    let ncode = b.get(4)? as usize + 4;
    if nlen > 286 || ndist > 30 {
        return Err(Error::BadData);
    }
    let mut cl = [0u8; 19];
    for &i in ORDER.iter().take(ncode) {
        cl[i] = b.get(3)? as u8;
    }
    let clh = Huff::build(&cl)?;
    let mut lens = [0u8; 320];
    let total = nlen + ndist;
    let mut i = 0;
    while i < total {
        let s = clh.decode(b)?;
        let (val, rep) = match s {
            0..=15 => (s as u8, 1),
            16 => {
                if i == 0 {
                    return Err(Error::BadData);
                }
                (lens[i - 1], 3 + b.get(2)? as usize)
            }
            17 => (0, 3 + b.get(3)? as usize),
            _ => (0, 11 + b.get(7)? as usize),
        };
        if i + rep > total {
            return Err(Error::BadData);
        }
        for _ in 0..rep {
            lens[i] = val;
            i += 1;
        }
    }
    if lens[256] == 0 {
        return Err(Error::BadData);
    }
    Ok((Huff::build(&lens[..nlen])?, Huff::build(&lens[nlen..total])?))
}

fn codes(b: &mut Bits, o: &mut Out, lit: &Huff, dist: &Huff) -> Result<(), Error> {
    loop {
        let s = lit.decode(b)? as usize;
        if s < 256 {
            o.put(s as u8)?;
        } else if s == 256 {
            return Ok(());
        } else {
            let li = s - 257;
            if li >= 29 {
                return Err(Error::BadData);
            }
            let len = LBASE[li] as usize + b.get(LEXT[li] as u32)? as usize;
            let di = dist.decode(b)? as usize;
            if di >= 30 {
                return Err(Error::BadData);
            }
            let d = DBASE[di] as usize + b.get(DEXT[di] as u32)? as usize;
            if d > o.n {
                return Err(Error::BadData);
            }
            for _ in 0..len {
                let x = o.buf[o.n - d];
                o.put(x)?;
            }
        }
    }
}
