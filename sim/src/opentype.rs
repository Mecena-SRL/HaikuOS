//! Lettore OpenType/TrueType in sola lettura (solo host): famiglia, unitsPerEm, numero di glifi.
//! Ogni accesso è controllato sui limiti del file. Vedi `docs/font-manager.md`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OtError {
    Truncated,
    BadVersion,
    BadMagic,
    MissingTable([u8; 4]),
    NoFamilyName,
    Composite,
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

/// Contorno: bounding box in unità del font e contorni (x, y, su_curva).
#[derive(Debug, Clone, PartialEq)]
pub struct Outline {
    pub contours: Vec<Vec<(i32, i32, bool)>>,
}

/// Metriche globali da `head` e `hhea`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metrics {
    pub units_per_em: u16,
    pub y_min: i32,
    pub y_max: i32,
}

pub fn metrics(data: &[u8]) -> Result<Metrics, OtError> {
    let head = table(data, *b"head")?;
    Ok(Metrics {
        units_per_em: u16_at(head, 18)?,
        y_min: i16_at(head, 38)? as i32,
        y_max: i16_at(head, 42)? as i32,
    })
}

fn i16_at(d: &[u8], o: usize) -> Result<i16, OtError> {
    Ok(u16_at(d, o)? as i16)
}

/// Indice del glifo per un carattere (cmap formato 4, BMP). 0 = glifo mancante.
pub fn glyph_id(data: &[u8], ch: char) -> Result<u16, OtError> {
    let cmap = table(data, *b"cmap")?;
    let n = u16_at(cmap, 2)? as usize;
    for i in 0..n {
        let r = 4 + i * 8;
        let pid = u16_at(cmap, r)?;
        let eid = u16_at(cmap, r + 2)?;
        if !((pid == 3 && eid == 1) || pid == 0) {
            continue;
        }
        let off = u32_at(cmap, r + 4)? as usize;
        let sub = cmap.get(off..).ok_or(OtError::Truncated)?;
        if u16_at(sub, 0)? == 4 {
            return format4(sub, ch);
        }
    }
    Ok(0)
}

fn format4(s: &[u8], ch: char) -> Result<u16, OtError> {
    let c = match u16::try_from(ch as u32) {
        Ok(c) => c,
        Err(_) => return Ok(0),
    };
    let seg2 = u16_at(s, 6)? as usize;
    let ends = 14;
    let starts = ends + seg2 + 2;
    let deltas = starts + seg2;
    let ranges = deltas + seg2;
    for i in 0..seg2 / 2 {
        if c > u16_at(s, ends + 2 * i)? {
            continue;
        }
        let start = u16_at(s, starts + 2 * i)?;
        if c < start {
            return Ok(0);
        }
        let delta = u16_at(s, deltas + 2 * i)?;
        let ro = u16_at(s, ranges + 2 * i)? as usize;
        if ro == 0 {
            return Ok(c.wrapping_add(delta));
        }
        let g = u16_at(s, ranges + 2 * i + ro + 2 * (c - start) as usize)?;
        return Ok(if g == 0 { 0 } else { g.wrapping_add(delta) });
    }
    Ok(0)
}

/// Larghezza di avanzamento di un glifo (hmtx), in unità del font.
pub fn advance(data: &[u8], gid: u16) -> Result<u16, OtError> {
    let hhea = table(data, *b"hhea")?;
    let n = u16_at(hhea, 34)? as usize;
    let hmtx = table(data, *b"hmtx")?;
    let i = (gid as usize).min(n.saturating_sub(1));
    u16_at(hmtx, 4 * i)
}

/// Contorno di un glifo semplice (`None` se vuoto, es. spazio). Glifi composti: errore.
pub fn outline(data: &[u8], gid: u16) -> Result<Option<Outline>, OtError> {
    let long = i16_at(table(data, *b"head")?, 50)? != 0;
    let loca = table(data, *b"loca")?;
    let g = gid as usize;
    let (start, end) = if long {
        (u32_at(loca, 4 * g)? as usize, u32_at(loca, 4 * g + 4)? as usize)
    } else {
        (2 * u16_at(loca, 2 * g)? as usize, 2 * u16_at(loca, 2 * g + 2)? as usize)
    };
    if start == end {
        return Ok(None);
    }
    let gl = table(data, *b"glyf")?.get(start..end).ok_or(OtError::Truncated)?;
    let nc = i16_at(gl, 0)?;
    if nc < 0 {
        return Err(OtError::Composite);
    }
    let nc = nc as usize;
    let mut ends = Vec::with_capacity(nc);
    for i in 0..nc {
        ends.push(u16_at(gl, 10 + 2 * i)? as usize);
    }
    let npts = ends.last().map_or(0, |e| e + 1);
    let ilen = u16_at(gl, 10 + 2 * nc)? as usize;
    let mut p = 12 + 2 * nc + ilen;
    let mut flags = Vec::with_capacity(npts);
    while flags.len() < npts {
        let f = *gl.get(p).ok_or(OtError::Truncated)?;
        p += 1;
        flags.push(f);
        if f & 8 != 0 {
            let r = *gl.get(p).ok_or(OtError::Truncated)?;
            p += 1;
            for _ in 0..r {
                if flags.len() < npts {
                    flags.push(f);
                }
            }
        }
    }
    let xs = coords(gl, &mut p, &flags, 2, 16)?;
    let ys = coords(gl, &mut p, &flags, 4, 32)?;
    let mut contours = Vec::with_capacity(nc);
    let mut s = 0;
    for &e in &ends {
        if e < s || e >= npts {
            return Err(OtError::Truncated);
        }
        contours.push((s..=e).map(|i| (xs[i], ys[i], flags[i] & 1 != 0)).collect());
        s = e + 1;
    }
    Ok(Some(Outline { contours }))
}

/// Coordinate x o y: 1 byte se `short`, altrimenti 2 byte; bit `same` = stesso valore o positivo.
fn coords(gl: &[u8], p: &mut usize, flags: &[u8], short: u8, same: u8) -> Result<Vec<i32>, OtError> {
    let mut v = 0i32;
    let mut out = Vec::with_capacity(flags.len());
    for &f in flags {
        if f & short != 0 {
            let d = *gl.get(*p).ok_or(OtError::Truncated)? as i32;
            *p += 1;
            v += if f & same != 0 { d } else { -d };
        } else if f & same == 0 {
            v += i16_at(gl, *p)? as i32;
            *p += 2;
        }
        out.push(v);
    }
    Ok(out)
}

/// Costruisce un file sintetico valido (solo per i test): head, maxp, name, cmap, hhea, hmtx,
/// loca, glyf. Glifo 0 vuoto; glifo 1 = quadrato 'A' (x 100..600, y 0..700), avanzo 600.
pub fn synth_font(family: &str) -> Vec<u8> {
    let mut head = vec![0u8; 54];
    head[12..16].copy_from_slice(&HEAD_MAGIC.to_be_bytes());
    head[18..20].copy_from_slice(&1000u16.to_be_bytes());
    head[42..44].copy_from_slice(&1000i16.to_be_bytes()); // yMax
    let mut maxp = vec![0u8; 6];
    maxp[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    maxp[4..6].copy_from_slice(&2u16.to_be_bytes());
    let utf16: Vec<u8> = family.encode_utf16().flat_map(|u| u.to_be_bytes()).collect();
    let mut name = Vec::new();
    for v in [0u16, 1, 18] {
        name.extend_from_slice(&v.to_be_bytes());
    }
    for v in [3u16, 1, 0x409, 1, utf16.len() as u16, 0] {
        name.extend_from_slice(&v.to_be_bytes());
    }
    name.extend_from_slice(&utf16);
    let mut cmap = Vec::new();
    for v in [0u16, 1, 3, 1] {
        cmap.extend_from_slice(&v.to_be_bytes());
    }
    cmap.extend_from_slice(&12u32.to_be_bytes());
    let mut f4 = Vec::new();
    for v in [4u16, 32, 0, 4, 0, 0, 0, 0x41, 0xFFFF, 0, 0x41, 0xFFFF] {
        f4.extend_from_slice(&v.to_be_bytes());
    }
    for v in [1u16.wrapping_sub(0x41), 1, 0, 0] {
        f4.extend_from_slice(&v.to_be_bytes()); // idDelta, idRangeOffset
    }
    cmap.extend_from_slice(&f4);
    let mut hhea = vec![0u8; 36];
    hhea[34..36].copy_from_slice(&2u16.to_be_bytes());
    let mut hmtx = Vec::new();
    for adv in [500u16, 600] {
        hmtx.extend_from_slice(&adv.to_be_bytes());
        hmtx.extend_from_slice(&0i16.to_be_bytes());
    }
    let mut glyf = Vec::new();
    glyf.extend_from_slice(&1i16.to_be_bytes()); // contorni
    for v in [100i16, 0, 600, 700] {
        glyf.extend_from_slice(&v.to_be_bytes()); // bbox
    }
    glyf.extend_from_slice(&3u16.to_be_bytes()); // fine contorno
    glyf.extend_from_slice(&0u16.to_be_bytes()); // istruzioni
    glyf.extend_from_slice(&[1, 1, 1, 1]); // tutti su curva, coordinate a 16 bit
    for v in [100i16, 500, 0, -500, 0, 0, 700, 0] {
        glyf.extend_from_slice(&v.to_be_bytes());
    }
    let glyf_len = glyf.len();
    let mut loca = Vec::new();
    for v in [0u16, 0, (glyf_len / 2) as u16] {
        loca.extend_from_slice(&v.to_be_bytes()); // formato corto
    }
    let tables: [([u8; 4], Vec<u8>); 8] = [
        (*b"head", head),
        (*b"maxp", maxp),
        (*b"name", name),
        (*b"cmap", cmap),
        (*b"hhea", hhea),
        (*b"hmtx", hmtx),
        (*b"loca", loca),
        (*b"glyf", glyf),
    ];
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
