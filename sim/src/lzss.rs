//! Compressione LZSS a finestra piccola (stile heatshrink) per la RAM di uno slot.
//! Decodifica: usa solo la finestra (256 byte) e l'output, niente tabelle.
//! Formato: 4 byte di lunghezza (LE), poi un flusso di bit:
//!   1 + 8 bit  = byte letterale;
//!   0 + 8 bit (distanza-1) + 3 bit (lunghezza-3) = copia dalla finestra.
//! Da verificare: rapporto di compressione su dati reali del GBA.

pub const WINDOW: usize = 256;
pub const MIN_MATCH: usize = 3;
pub const MAX_MATCH: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LzssError {
    /// Lunghezza dichiarata oltre il limite del chiamante.
    TooLarge,
    /// Dati finiti prima della fine o copia fuori finestra.
    Corrupt,
}

struct Bits {
    out: Vec<u8>,
    n: usize,
}

impl Bits {
    fn put(&mut self, bit: bool) {
        if self.n % 8 == 0 {
            self.out.push(0);
        }
        if bit {
            *self.out.last_mut().unwrap() |= 0x80 >> (self.n % 8);
        }
        self.n += 1;
    }
    fn put_n(&mut self, v: usize, bits: u32) {
        for i in (0..bits).rev() {
            self.put((v >> i) & 1 == 1);
        }
    }
}

pub fn compress(input: &[u8]) -> Vec<u8> {
    let len = input.len();
    let mut w = Bits { out: (len as u32).to_le_bytes().to_vec(), n: 0 };
    let mut pos = 0;
    while pos < len {
        let mut best = (0usize, 0usize); // (distanza, lunghezza)
        for cand in pos.saturating_sub(WINDOW)..pos {
            let mut l = 0;
            while l < MAX_MATCH && pos + l < len && input[cand + l] == input[pos + l] {
                l += 1;
            }
            if l >= MIN_MATCH && l > best.1 {
                best = (pos - cand, l);
            }
        }
        if best.1 > 0 {
            w.put(false);
            w.put_n(best.0 - 1, 8);
            w.put_n(best.1 - MIN_MATCH, 3);
            pos += best.1;
        } else {
            w.put(true);
            w.put_n(input[pos] as usize, 8);
            pos += 1;
        }
    }
    w.out
}

pub fn decompress(data: &[u8], max_out: usize) -> Result<Vec<u8>, LzssError> {
    if data.len() < 4 {
        return Err(LzssError::Corrupt);
    }
    let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    if len > max_out {
        return Err(LzssError::TooLarge);
    }
    let body = &data[4..];
    let mut bit = 0usize;
    let mut read = |bits: u32| -> Result<usize, LzssError> {
        let mut v = 0;
        for _ in 0..bits {
            let byte = *body.get(bit / 8).ok_or(LzssError::Corrupt)?;
            v = (v << 1) | ((byte >> (7 - bit % 8)) & 1) as usize;
            bit += 1;
        }
        Ok(v)
    };
    let mut out: Vec<u8> = Vec::with_capacity(len);
    while out.len() < len {
        if read(1)? == 1 {
            out.push(read(8)? as u8);
        } else {
            let dist = read(8)? + 1;
            let l = read(3)? + MIN_MATCH;
            if dist > out.len() || out.len() + l > len {
                return Err(LzssError::Corrupt);
            }
            for _ in 0..l {
                out.push(out[out.len() - dist]);
            }
        }
    }
    Ok(out)
}
