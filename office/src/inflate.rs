//! Decompressione DEFLATE (RFC 1951) in un buffer fisso. Solo `core`, nessuna allocazione.
//! Usata da `zip.rs` per aprire docx e odt. Restituisce None su dati corrotti o buffer troppo piccolo.

const MAXBITS: usize = 15;

struct Bits<'a> {
    src: &'a [u8],
    pos: usize,
    buf: u32,
    cnt: u32,
}

impl<'a> Bits<'a> {
    fn bit(&mut self) -> Option<u32> {
        if self.cnt == 0 {
            let b = *self.src.get(self.pos)?;
            self.pos += 1;
            self.buf = b as u32;
            self.cnt = 8;
        }
        let v = self.buf & 1;
        self.buf >>= 1;
        self.cnt -= 1;
        Some(v)
    }

    fn bits(&mut self, n: u32) -> Option<u32> {
        let mut v = 0;
        for i in 0..n {
            v |= self.bit()? << i;
        }
        Some(v)
    }

    fn align(&mut self) {
        self.cnt = 0;
    }
}

/// Tabella Huffman canonica: conteggi per lunghezza e simboli ordinati.
struct Huff {
    count: [u16; MAXBITS + 1],
    symbol: [u16; 320],
}

impl Huff {
    fn new(lengths: &[u8]) -> Option<Huff> {
        let mut h = Huff { count: [0; MAXBITS + 1], symbol: [0; 320] };
        for &l in lengths {
            h.count[l as usize] += 1;
        }
        // Codici non validi: sovrasottoscritto.
        let mut left: i32 = 1;
        for len in 1..=MAXBITS {
            left <<= 1;
            left -= h.count[len] as i32;
            if left < 0 {
                return None;
            }
        }
        let mut offs = [0usize; MAXBITS + 1];
        for len in 1..MAXBITS {
            offs[len + 1] = offs[len] + h.count[len] as usize;
        }
        for (sym, &l) in lengths.iter().enumerate() {
            if l != 0 {
                h.symbol[offs[l as usize]] = sym as u16;
                offs[l as usize] += 1;
            }
        }
        Some(h)
    }

    fn decode(&self, br: &mut Bits) -> Option<u16> {
        let mut code: i32 = 0;
        let mut first: i32 = 0;
        let mut index: i32 = 0;
        for len in 1..=MAXBITS {
            code |= br.bit()? as i32;
            let count = self.count[len] as i32;
            if code - count < first {
                return self.symbol.get((index + (code - first)) as usize).copied();
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        None
    }
}

const LBASE: [u16; 29] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LEXT: [u8; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DBASE: [u16; 30] = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577];
const DEXT: [u8; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];

/// Decomprime `src` (DEFLATE grezzo) dentro `out`. Ritorna i byte scritti.
pub fn inflate(src: &[u8], out: &mut [u8]) -> Option<usize> {
    let mut br = Bits { src, pos: 0, buf: 0, cnt: 0 };
    let mut o = 0usize;
    loop {
        let last = br.bit()?;
        match br.bits(2)? {
            0 => {
                br.align();
                let len = u16::from_le_bytes([*src.get(br.pos)?, *src.get(br.pos + 1)?]) as usize;
                br.pos += 4; // LEN e NLEN
                let chunk = src.get(br.pos..br.pos + len)?;
                out.get_mut(o..o + len)?.copy_from_slice(chunk);
                o += len;
                br.pos += len;
            }
            1 => {
                let mut l = [0u8; 288];
                for (i, v) in l.iter_mut().enumerate() {
                    *v = match i {
                        0..=143 => 8,
                        144..=255 => 9,
                        256..=279 => 7,
                        _ => 8,
                    };
                }
                let lh = Huff::new(&l)?;
                let dh = Huff::new(&[5u8; 30])?;
                o = codes(&mut br, &lh, &dh, out, o)?;
            }
            2 => {
                let nlen = br.bits(5)? as usize + 257;
                let ndist = br.bits(5)? as usize + 1;
                let ncode = br.bits(4)? as usize + 4;
                const ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
                let mut cl = [0u8; 19];
                for &idx in ORDER.iter().take(ncode) {
                    cl[idx] = br.bits(3)? as u8;
                }
                let ch = Huff::new(&cl)?;
                let mut lens = [0u8; 320];
                let mut i = 0;
                while i < nlen + ndist {
                    let sym = ch.decode(&mut br)?;
                    match sym {
                        0..=15 => {
                            lens[i] = sym as u8;
                            i += 1;
                        }
                        16 => {
                            let prev = *lens.get(i.checked_sub(1)?)?;
                            for _ in 0..3 + br.bits(2)? {
                                *lens.get_mut(i)? = prev;
                                i += 1;
                            }
                        }
                        17 => {
                            for _ in 0..3 + br.bits(3)? {
                                *lens.get_mut(i)? = 0;
                                i += 1;
                            }
                        }
                        _ => {
                            for _ in 0..11 + br.bits(7)? {
                                *lens.get_mut(i)? = 0;
                                i += 1;
                            }
                        }
                    }
                    if i > nlen + ndist {
                        return None;
                    }
                }
                let lh = Huff::new(&lens[..nlen])?;
                let dh = Huff::new(&lens[nlen..nlen + ndist])?;
                o = codes(&mut br, &lh, &dh, out, o)?;
            }
            _ => return None,
        }
        if last == 1 {
            return Some(o);
        }
    }
}

fn codes(br: &mut Bits, lh: &Huff, dh: &Huff, out: &mut [u8], mut o: usize) -> Option<usize> {
    loop {
        let sym = lh.decode(br)? as usize;
        if sym < 256 {
            *out.get_mut(o)? = sym as u8;
            o += 1;
        } else if sym == 256 {
            return Some(o);
        } else {
            let li = sym - 257;
            let len = *LBASE.get(li)? as usize + br.bits(*LEXT.get(li)? as u32)? as usize;
            let di = dh.decode(br)? as usize;
            let dist = *DBASE.get(di)? as usize + br.bits(*DEXT.get(di)? as u32)? as usize;
            if dist > o {
                return None;
            }
            for _ in 0..len {
                *out.get_mut(o)? = out[o - dist];
                o += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dinamico() {
        let mut out = [0u8; 8192];
        let n = inflate(include_bytes!("../tests/fixtures/dyn.bin"), &mut out).unwrap();
        assert_eq!(&out[..n], include_bytes!("../tests/fixtures/dyn.txt"));
    }

    #[test]
    fn fisso() {
        let mut out = [0u8; 64];
        let n = inflate(include_bytes!("../tests/fixtures/fixed.bin"), &mut out).unwrap();
        assert_eq!(&out[..n], b"ciao ciao ciao haiku");
    }

    #[test]
    fn stored() {
        let mut out = [0u8; 64];
        let n = inflate(include_bytes!("../tests/fixtures/stored.bin"), &mut out).unwrap();
        assert_eq!(&out[..n], b"dati non compressi");
    }

    #[test]
    fn buffer_piccolo_e_corrotto() {
        let mut small = [0u8; 10];
        assert!(inflate(include_bytes!("../tests/fixtures/dyn.bin"), &mut small).is_none());
        assert!(inflate(&[0xFF, 0xFF, 0xFF], &mut [0u8; 64]).is_none());
    }
}
