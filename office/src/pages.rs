//! Pages (.pages): testo di `Index/Document.iwa`. Il file IWA è una sequenza di chunk
//! (1 byte di tipo, 3 byte di lunghezza LE, blocco Snappy senza CRC). Ogni chunk contiene
//! record: intestazione `ArchiveInfo` (varint + protobuf), poi i payload in ordine.
//! Il testo sta nei payload di tipo 2001 (TSWP.StorageArchive), campo 3 (stringhe).
//! **Da verificare** su file Pages reali: tipo 2001 e campo 3 sono letti dalla specifica
//! pubblica dei formati (iWork), non confrontati con un file Apple.

use crate::snappy::decompress;
use crate::zip::extract;

const STORAGE_TYPE: usize = 2001;
const TEXT_FIELD: u32 = 3;

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

/// Campi protobuf: (numero, valore varint o byte). Gli altri tipi di campo finiscono la lettura.
enum Val<'a> {
    Var(usize),
    Bytes(&'a [u8]),
}

fn fields(b: &[u8]) -> impl Iterator<Item = (u32, Val<'_>)> {
    let mut i = 0usize;
    core::iter::from_fn(move || {
        if i >= b.len() {
            return None;
        }
        let (key, j) = varint(b, i)?;
        let (num, wt) = ((key >> 3) as u32, key & 7);
        match wt {
            0 => {
                let (v, k) = varint(b, j)?;
                i = k;
                Some((num, Val::Var(v)))
            }
            2 => {
                let (len, k) = varint(b, j)?;
                let v = b.get(k..k + len)?;
                i = k + len;
                Some((num, Val::Bytes(v)))
            }
            1 => {
                i = j + 8;
                None
            }
            5 => {
                i = j + 4;
                None
            }
            _ => {
                i = b.len();
                None
            }
        }
    })
}

/// Un payload di tipo StorageArchive: ogni stringa del campo 3 va in `out`, seguita da LF.
fn storage(payload: &[u8], out: &mut [u8], o: &mut usize) -> Option<()> {
    for (num, v) in fields(payload) {
        if let (TEXT_FIELD, Val::Bytes(s)) = (num, v) {
            out.get_mut(*o..*o + s.len())?.copy_from_slice(s);
            *o += s.len();
            *out.get_mut(*o)? = b'\n';
            *o += 1;
        }
    }
    Some(())
}

/// Testo di un file `Index/Document.iwa` già estratto dallo zip. `buf` riceve il contenuto
/// decompresso (grande quanto il documento), `out` il testo.
pub fn iwa_text(iwa: &[u8], buf: &mut [u8], out: &mut [u8]) -> Option<usize> {
    // 1. Decomprimi i chunk nel buffer.
    let (mut n, mut i) = (0usize, 0usize);
    while i + 4 <= iwa.len() {
        let len = u32::from_le_bytes([iwa[i + 1], iwa[i + 2], iwa[i + 3], 0]) as usize;
        i += 4;
        let chunk = iwa.get(i..i + len)?;
        n += decompress(chunk, buf.get_mut(n..)?)?;
        i += len;
    }
    // 2. Record: intestazione, poi payload in ordine.
    let (mut pos, mut o) = (0usize, 0usize);
    while pos < n {
        let (hl, p) = varint(buf, pos)?;
        let head = buf.get(p..p + hl)?;
        let mut cur = p + hl;
        for (num, v) in fields(head) {
            // ArchiveInfo campo 2: MessageInfo { 1: tipo, 3: lunghezza }.
            if let (2, Val::Bytes(mi)) = (num, v) {
                let (mut ty, mut len) = (0usize, 0usize);
                for (k, v) in fields(mi) {
                    match (k, v) {
                        (1, Val::Var(x)) => ty = x,
                        (3, Val::Var(x)) => len = x,
                        _ => {}
                    }
                }
                let payload = buf.get(cur..cur + len)?;
                if ty == STORAGE_TYPE {
                    storage(payload, out, &mut o)?;
                }
                cur += len;
            }
        }
        pos = cur;
    }
    Some(o)
}

/// Testo di un file `.pages` (archivio zip). `buf` e `out` devono essere abbastanza grandi.
pub fn pages_text(zip: &[u8], iwa_buf: &mut [u8], iwa: &mut [u8], out: &mut [u8]) -> Option<usize> {
    let n = extract(zip, b"Index/Document.iwa", iwa)?;
    iwa_text(&iwa[..n], iwa_buf, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_di_prova() {
        let zip = include_bytes!("../tests/fixtures/sample.pages");
        let (mut buf, mut iwa, mut out) = ([0u8; 4096], [0u8; 4096], [0u8; 512]);
        let n = pages_text(zip, &mut buf, &mut iwa, &mut out).unwrap();
        assert_eq!(&out[..n], "Ciao mondo\nSeconda riga\n".as_bytes());
    }
}
