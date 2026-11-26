//! Decompressione Snappy (formato a blocco, come nella specifica di Google). Solo `core`.
//! Nessun CRC: il chiamante sa la lunghezza. `None` se il blocco è malformato o non entra in `out`.

fn varint(b: &[u8], mut i: usize) -> Option<(usize, usize)> {
    let (mut v, mut shift) = (0usize, 0u32);
    loop {
        let x = *b.get(i)?;
        i += 1;
        v |= ((x & 0x7f) as usize).checked_shl(shift)?;
        if x & 0x80 == 0 {
            return Some((v, i));
        }
        shift += 7;
        if shift > 28 {
            return None;
        }
    }
}

fn le(b: &[u8], i: usize, n: usize) -> Option<usize> {
    let mut v = 0usize;
    for k in 0..n {
        v |= (*b.get(i + k)? as usize) << (8 * k);
    }
    Some(v)
}

/// Copia `len` byte da `off` indietro. Ammesse copie sovrapposte (`off < len`).
fn copy(out: &mut [u8], o: &mut usize, off: usize, len: usize) -> Option<()> {
    if off == 0 || off > *o || *o + len > out.len() {
        return None;
    }
    for _ in 0..len {
        out[*o] = out[*o - off];
        *o += 1;
    }
    Some(())
}

/// Decomprime un blocco Snappy in `out`. Ritorna i byte scritti.
pub fn decompress(src: &[u8], out: &mut [u8]) -> Option<usize> {
    let (ulen, mut i) = varint(src, 0)?;
    if ulen > out.len() {
        return None;
    }
    let mut o = 0usize;
    while i < src.len() {
        let tag = src[i];
        i += 1;
        match tag & 3 {
            0 => {
                let mut len = (tag >> 2) as usize;
                if len >= 60 {
                    let nb = len - 59;
                    len = le(src, i, nb)?;
                    i += nb;
                }
                len += 1;
                let lit = src.get(i..i + len)?;
                if o + len > out.len() {
                    return None;
                }
                out[o..o + len].copy_from_slice(lit);
                o += len;
                i += len;
            }
            1 => {
                let len = ((tag >> 2) & 7) as usize + 4;
                let off = ((tag >> 5) as usize) << 8 | *src.get(i)? as usize;
                i += 1;
                copy(out, &mut o, off, len)?;
            }
            2 => {
                let len = (tag >> 2) as usize + 1;
                let off = le(src, i, 2)?;
                i += 2;
                copy(out, &mut o, off, len)?;
            }
            _ => {
                let len = (tag >> 2) as usize + 1;
                let off = le(src, i, 4)?;
                i += 4;
                copy(out, &mut o, off, len)?;
            }
        }
    }
    (o == ulen).then_some(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letterale() {
        let mut out = [0u8; 16];
        let n = decompress(&[0x05, 0x10, b'h', b'e', b'l', b'l', b'o'], &mut out).unwrap();
        assert_eq!(&out[..n], b"hello");
    }

    #[test]
    fn copia_due_byte_e_sovrapposta() {
        let mut out = [0u8; 16];
        // "abc" poi copia (off 3, len 6) con tag di tipo 2 = "abcabcabc"
        let n = decompress(&[0x09, 0x08, b'a', b'b', b'c', 0x16, 0x03, 0x00], &mut out).unwrap();
        assert_eq!(&out[..n], b"abcabcabc");
        // "a" poi copia sovrapposta (off 1, len 9) = dieci "a"
        let n = decompress(&[0x0a, 0x00, b'a', 0x22, 0x01, 0x00], &mut out).unwrap();
        assert_eq!(&out[..n], b"aaaaaaaaaa");
    }

    #[test]
    fn copia_un_byte_di_offset() {
        let mut out = [0u8; 16];
        // "abc" poi copia tipo 1 (off 3, len 6)
        let n = decompress(&[0x09, 0x08, b'a', b'b', b'c', 0x09, 0x03], &mut out).unwrap();
        assert_eq!(&out[..n], b"abcabcabc");
    }

    #[test]
    fn errori() {
        let mut out = [0u8; 16];
        assert!(decompress(&[0x05, 0x10, b'h'], &mut out).is_none()); // troncato
        assert!(decompress(&[0x02, 0x01, 0x00, 0x00], &mut out).is_none()); // offset oltre l'inizio
        assert!(decompress(&[0x09, 0x10, b'h'], &mut out).is_none()); // lunghezza non corrisponde
    }
}
