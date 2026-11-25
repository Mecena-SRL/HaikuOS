//! Lettore OpenType/TrueType in sola lettura (solo host): famiglia, unitsPerEm, numero di glifi.
//! Ogni accesso è controllato sui limiti del file. Vedi `docs/font-manager.md`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OtError {
    Truncated,
    BadVersion,
    BadMagic,
    MissingTable([u8; 4]),
    NoFamilyName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontInfo {
    pub family: String,
    pub units_per_em: u16,
    pub num_glyphs: u16,
}

const HEAD_MAGIC: u32 = 0x5F0F_3CF5;

fn u16_at(d: &[u8], o: usize) -> Result<u16, OtError> {
    let b = d.get(o..o + 2).ok_or(OtError::Truncated)?;
    Ok(u16::from_be_bytes([b[0], b[1]]))
}

fn u32_at(d: &[u8], o: usize) -> Result<u32, OtError> {
    let b = d.get(o..o + 4).ok_or(OtError::Truncated)?;
    Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// Restituisce il contenuto di una tabella, controllando che stia nel file.
fn table<'a>(data: &'a [u8], tag: [u8; 4]) -> Result<&'a [u8], OtError> {
    let n = u16_at(data, 4)? as usize;
    for i in 0..n {
        let r = 12 + i * 16;
        if data.get(r..r + 4) == Some(&tag[..]) {
            let off = u32_at(data, r + 8)? as usize;
            let len = u32_at(data, r + 12)? as usize;
            let end = off.checked_add(len).ok_or(OtError::Truncated)?;
            return data.get(off..end).ok_or(OtError::Truncated);
        }
    }
    Err(OtError::MissingTable(tag))
}

/// Nome di famiglia (nameID 1): preferisce Windows (UTF-16BE), poi Macintosh (ASCII/Latin).
fn family_name(name: &[u8]) -> Result<String, OtError> {
    let count = u16_at(name, 2)? as usize;
    let store = u16_at(name, 4)? as usize;
    let mut mac = None;
    for i in 0..count {
        let r = 6 + i * 12;
        if u16_at(name, r + 6)? != 1 {
            continue;
        }
        let len = u16_at(name, r + 8)? as usize;
        let off = store + u16_at(name, r + 10)? as usize;
        let bytes = name.get(off..off + len).ok_or(OtError::Truncated)?;
        match u16_at(name, r)? {
            3 => {
                let units: Vec<u16> = bytes
                    .chunks_exact(2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();
                let s = String::from_utf16_lossy(&units);
                if !s.is_empty() {
                    return Ok(s);
                }
            }
            1 if mac.is_none() => mac = Some(String::from_utf8_lossy(bytes).into_owned()),
            _ => {}
        }
    }
    mac.filter(|s| !s.is_empty()).ok_or(OtError::NoFamilyName)
}

/// Legge le informazioni minime di un file OpenType/TrueType (in sola lettura).
pub fn parse(data: &[u8]) -> Result<FontInfo, OtError> {
    match u32_at(data, 0)? {
        0x0001_0000 | 0x4F54_544F | 0x7472_7565 => {} // TrueType, CFF ('OTTO'), Apple ('true')
        _ => return Err(OtError::BadVersion),
    }
    let head = table(data, *b"head")?;
    if u32_at(head, 12)? != HEAD_MAGIC {
        return Err(OtError::BadMagic);
    }
    let units_per_em = u16_at(head, 18)?;
    let maxp = table(data, *b"maxp")?;
    let num_glyphs = u16_at(maxp, 4)?;
    let family = family_name(table(data, *b"name")?)?;
    Ok(FontInfo { family, units_per_em, num_glyphs })
}

/// Costruisce un file sintetico valido (solo per i test): head, maxp, name con una famiglia.
pub fn synth_font(family: &str) -> Vec<u8> {
    let mut head = vec![0u8; 54];
    head[12..16].copy_from_slice(&HEAD_MAGIC.to_be_bytes());
    head[18..20].copy_from_slice(&1000u16.to_be_bytes());
    let mut maxp = vec![0u8; 6];
    maxp[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    maxp[4..6].copy_from_slice(&42u16.to_be_bytes());
    let utf16: Vec<u8> = family.encode_utf16().flat_map(|u| u.to_be_bytes()).collect();
    let mut name = Vec::new();
    for v in [0u16, 1, 18] {
        name.extend_from_slice(&v.to_be_bytes()); // formato, count, offset stringhe
    }
    for v in [3u16, 1, 0x409, 1, utf16.len() as u16, 0] {
        name.extend_from_slice(&v.to_be_bytes()); // record: piattaforma, codifica, lingua, nameID, lunghezza, offset
    }
    name.extend_from_slice(&utf16);
    let tables = [(*b"head", head), (*b"maxp", maxp), (*b"name", name)];
    let mut out = Vec::new();
    out.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    out.extend_from_slice(&(tables.len() as u16).to_be_bytes());
    out.extend_from_slice(&[0u8; 6]);
    let mut off = 12 + 16 * tables.len();
    let mut body = Vec::new();
    for (tag, data) in &tables {
        out.extend_from_slice(tag);
        out.extend_from_slice(&0u32.to_be_bytes());
        out.extend_from_slice(&(off as u32).to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        off += data.len();
        body.extend_from_slice(data);
    }
    out.extend_from_slice(&body);
    out
}
