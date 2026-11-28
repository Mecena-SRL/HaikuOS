//! Archivi ZIP per docx e odt. Lettura: metodo 0 (stored) e 8 (DEFLATE), tramite `inflate.rs`.
//! Scrittura: solo stored, con CRC-32. Nessuna allocazione, solo `core`.
//! Il CRC non viene verificato in lettura (**da verificare**: costo su ARM7 contro rischio di file corrotti).

use crate::inflate::inflate;

fn u16_at(b: &[u8], i: usize) -> Option<usize> {
    Some(u16::from_le_bytes([*b.get(i)?, *b.get(i + 1)?]) as usize)
}

fn u32_at(b: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_le_bytes([*b.get(i)?, *b.get(i + 1)?, *b.get(i + 2)?, *b.get(i + 3)?]) as usize)
}

/// Cerca `name` nell'archivio e decomprime il contenuto in `out`. Ritorna i byte scritti.
pub fn extract(zip: &[u8], name: &[u8], out: &mut [u8]) -> Option<usize> {
    // Fine dell'archivio: record EOCD (firma 0x06054b50), cerco dal fondo.
    let eocd = (0..=zip.len().checked_sub(22)?).rev().find(|&i| zip[i..i + 4] == [0x50, 0x4b, 0x05, 0x06])?;
    let entries = u16_at(zip, eocd + 10)?;
    let mut p = u32_at(zip, eocd + 16)?;
    for _ in 0..entries {
        if zip.get(p..p + 4)? != [0x50, 0x4b, 0x01, 0x02] {
            return None;
        }
        let method = u16_at(zip, p + 10)?;
        let csize = u32_at(zip, p + 20)?;
        let namelen = u16_at(zip, p + 28)?;
        let extra = u16_at(zip, p + 30)?;
        let comment = u16_at(zip, p + 32)?;
        let local = u32_at(zip, p + 42)?;
        let this = zip.get(p + 46..p + 46 + namelen)?;
        if this == name {
            // Intestazione locale (firma 0x04034b50): i campi di lunghezza sono quelli locali.
            if zip.get(local..local + 4)? != [0x50, 0x4b, 0x03, 0x04] {
                return None;
            }
            let lnamelen = u16_at(zip, local + 26)?;
            let lextra = u16_at(zip, local + 28)?;
            let start = local + 30 + lnamelen + lextra;
            let data = zip.get(start..start + csize)?;
            return match method {
                0 => {
                    out.get_mut(..data.len())?.copy_from_slice(data);
                    Some(data.len())
                }
                8 => inflate(data, out),
                _ => None,
            };
        }
        p += 46 + namelen + extra + comment;
    }
    None
}

/// CRC-32 (IEEE), usato nelle intestazioni dello ZIP scritto.
pub fn crc32(data: &[u8]) -> u32 {
    let mut c: u32 = 0xFFFF_FFFF;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
    }
    !c
}

fn put(out: &mut [u8], o: &mut usize, bytes: &[u8]) -> Option<()> {
    out.get_mut(*o..*o + bytes.len())?.copy_from_slice(bytes);
    *o += bytes.len();
    Some(())
}

/// Scrive un archivio ZIP con voci stored. Ritorna i byte scritti in `out`.
pub fn write(entries: &[(&[u8], &[u8])], out: &mut [u8]) -> Option<usize> {
    let mut o = 0;
    let mut central = [0u8; 2048];
    let mut cn = 0;
    for &(name, data) in entries {
        let local = o;
        let crc = crc32(data).to_le_bytes();
        let size = (data.len() as u32).to_le_bytes();
        let nlen = (name.len() as u16).to_le_bytes();
        put(out, &mut o, &[0x50, 0x4b, 0x03, 0x04, 20, 0, 0, 0, 0, 0, 0, 0, 0x21, 0])?;
        put(out, &mut o, &crc)?;
        put(out, &mut o, &size)?;
        put(out, &mut o, &size)?;
        put(out, &mut o, &nlen)?;
        put(out, &mut o, &[0, 0])?;
        put(out, &mut o, name)?;
        put(out, &mut o, data)?;
        // Voce della directory centrale: 46 byte fissi + nome.
        let mut c = [0u8; 46];
        c[..4].copy_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
        c[4..6].copy_from_slice(&[20, 0]);
        c[6..8].copy_from_slice(&[20, 0]);
        c[16..20].copy_from_slice(&crc);
        c[20..24].copy_from_slice(&size);
        c[24..28].copy_from_slice(&size);
        c[28..30].copy_from_slice(&nlen);
        c[42..46].copy_from_slice(&(local as u32).to_le_bytes());
        central.get_mut(cn..cn + 46)?.copy_from_slice(&c);
        central.get_mut(cn + 46..cn + 46 + name.len())?.copy_from_slice(name);
        cn += 46 + name.len();
    }
    let cd_start = o;
    put(out, &mut o, &central[..cn])?;
    let count = (entries.len() as u16).to_le_bytes();
    let mut e = [0u8; 22];
    e[..4].copy_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
    e[8..10].copy_from_slice(&count);
    e[10..12].copy_from_slice(&count);
    e[12..16].copy_from_slice(&(cn as u32).to_le_bytes());
    e[16..20].copy_from_slice(&(cd_start as u32).to_le_bytes());
    put(out, &mut o, &e)?;
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrivi_e_rileggi() {
        let mut buf = [0u8; 1024];
        let n = write(&[(b"mimetype", b"application/test"), (b"a/b.xml", b"<x>ciao</x>")], &mut buf).unwrap();
        let mut out = [0u8; 64];
        let k = extract(&buf[..n], b"a/b.xml", &mut out).unwrap();
        assert_eq!(&out[..k], b"<x>ciao</x>");
        assert!(extract(&buf[..n], b"manca", &mut out).is_none());
    }

    #[test]
    fn crc_noto() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn archivio_di_libreoffice() {
        let mut out = [0u8; 8192];
        let n = extract(include_bytes!("../tests/fixtures/sample.odt"), b"content.xml", &mut out).unwrap();
        assert!(out[..n].starts_with(b"<?xml"));
    }
}
