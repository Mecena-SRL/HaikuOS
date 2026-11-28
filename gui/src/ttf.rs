//! Lettore TrueType minimo. no_std, senza allocazione: cmap formato 4, hmtx, loca/glyf (glifi semplici).
//! Glifi composti, cmap diversi da format 4 e tabelle mancanti danno `None`.

pub const MAX_PTS: usize = 256;
pub const MAX_CONTOURS: usize = 24;

fn u16_at(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*d.get(o)?, *d.get(o + 1)?]))
}
fn i16_at(d: &[u8], o: usize) -> Option<i16> {
    Some(u16_at(d, o)? as i16)
}
fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes([*d.get(o)?, *d.get(o + 1)?, *d.get(o + 2)?, *d.get(o + 3)?]))
}

/// Offset e lunghezza di una tabella (per tag).
fn table(d: &[u8], tag: &[u8; 4]) -> Option<usize> {
    let n = u16_at(d, 4)? as usize;
    for i in 0..n {
        let r = 12 + 16 * i;
        if d.get(r..r + 4)? == tag {
            return Some(u32_at(d, r + 8)? as usize);
        }
    }
    None
}

/// Punto di contorno in unità del font; `on` = punto su curva.
#[derive(Clone, Copy)]
pub struct Pt {
    pub x: i32,
    pub y: i32,
    pub on: bool,
}

/// Glifo semplice: punti e fine di ogni contorno (indici in `pts`).
pub struct Glyph {
    pub pts: [Pt; MAX_PTS],
    pub n: usize,
    pub ends: [usize; MAX_CONTOURS],
    pub nc: usize,
}

#[derive(Clone, Copy)]
pub struct Font {
    d: &'static [u8],
    pub upem: i32,
    /// Ascendente in unità del font (hhea): distanza della linea di base dal bordo alto.
    pub ascender: i32,
    loca: usize,
    glyf: usize,
    hmtx: usize,
    n_hmetrics: usize,
    cmap: usize,
    long_loca: bool,
}

impl Font {
    pub fn parse(d: &'static [u8]) -> Option<Font> {
        let head = table(d, b"head")?;
        if u32_at(d, head + 12)? != 0x5F0F_3CF5 {
            return None;
        }
        let hhea = table(d, b"hhea")?;
        let cm = table(d, b"cmap")?;
        let mut cmap = None;
        for i in 0..u16_at(d, cm + 2)? as usize {
            let r = cm + 4 + 8 * i;
            let (pid, eid) = (u16_at(d, r)?, u16_at(d, r + 2)?);
            if (pid == 3 && eid == 1) || pid == 0 {
                let off = cm + u32_at(d, r + 4)? as usize;
                if u16_at(d, off)? == 4 {
                    cmap = Some(off);
                    break;
                }
            }
        }
        Some(Font {
            d,
            upem: u16_at(d, head + 18)? as i32,
            ascender: i16_at(d, hhea + 4)? as i32,
            loca: table(d, b"loca")?,
            glyf: table(d, b"glyf")?,
            hmtx: table(d, b"hmtx")?,
            n_hmetrics: u16_at(d, hhea + 34)? as usize,
            cmap: cmap?,
            long_loca: i16_at(d, head + 50)? != 0,
        })
    }

    /// Indice del glifo per un codice Unicode (0 = mancante).
    pub fn glyph_id(&self, c: u32) -> u16 {
        self.lookup(c).unwrap_or(0)
    }

    fn lookup(&self, c: u32) -> Option<u16> {
        let (d, s) = (self.d, self.cmap);
        let c = u16::try_from(c).ok()?;
        let seg2 = u16_at(d, s + 6)? as usize;
        let ends = s + 14;
        let starts = ends + seg2 + 2;
        let deltas = starts + seg2;
        let ranges = deltas + seg2;
        for i in 0..seg2 / 2 {
            if c > u16_at(d, ends + 2 * i)? {
                continue;
            }
            let st = u16_at(d, starts + 2 * i)?;
            if c < st {
                return Some(0);
            }
            let delta = u16_at(d, deltas + 2 * i)?;
            let ro = u16_at(d, ranges + 2 * i)? as usize;
            if ro == 0 {
                return Some(c.wrapping_add(delta));
            }
            let g = u16_at(d, ranges + 2 * i + ro + 2 * (c - st) as usize)?;
            return Some(if g == 0 { 0 } else { g.wrapping_add(delta) });
        }
        Some(0)
    }

    /// Avanzo orizzontale del glifo, in unità del font.
    pub fn advance(&self, gid: u16) -> i32 {
        let i = (gid as usize).min(self.n_hmetrics.saturating_sub(1));
        u16_at(self.d, self.hmtx + 4 * i).unwrap_or(0) as i32
    }

    /// Contorni del glifo, composti inclusi (solo offset; scala ignorata). `None` se vuoto o troppo grande.
    pub fn glyph(&self, gid: u16) -> Option<Glyph> {
        let mut out = Glyph { pts: [Pt { x: 0, y: 0, on: false }; MAX_PTS], n: 0, ends: [0; MAX_CONTOURS], nc: 0 };
        self.append(gid, &mut out, 0, 0, 0)?;
        if out.n == 0 { None } else { Some(out) }
    }

    fn append(&self, gid: u16, out: &mut Glyph, dx: i32, dy: i32, depth: u8) -> Option<()> {
        let (d, g) = (self.d, gid as usize);
        if depth > 4 {
            return None;
        }
        let (s, e) = if self.long_loca {
            (u32_at(d, self.loca + 4 * g)? as usize, u32_at(d, self.loca + 4 * g + 4)? as usize)
        } else {
            (2 * u16_at(d, self.loca + 2 * g)? as usize, 2 * u16_at(d, self.loca + 2 * g + 2)? as usize)
        };
        if e <= s {
            return Some(());
        }
        let p0 = self.glyf + s;
        let nc = i16_at(d, p0)?;
        if nc < 0 {
            return self.composite(p0 + 10, out, dx, dy, depth);
        }
        self.simple(p0, nc as usize, out, dx, dy)
    }

    /// Componenti di un glifo composto: ogni componente è spostato di (ox, oy) e aggiunto in coda.
    fn composite(&self, mut p: usize, out: &mut Glyph, dx: i32, dy: i32, depth: u8) -> Option<()> {
        let d = self.d;
        loop {
            let flags = u16_at(d, p)?;
            let idx = u16_at(d, p + 2)?;
            p += 4;
            if flags & 2 == 0 {
                return None; // ancoraggio a punti: non supportato
            }
            let (ox, oy) = if flags & 1 != 0 {
                let r = (i16_at(d, p)? as i32, i16_at(d, p + 2)? as i32);
                p += 4;
                r
            } else {
                let r = (*d.get(p)? as i8 as i32, *d.get(p + 1)? as i8 as i32);
                p += 2;
                r
            };
            p += match flags & 0xC8 {
                0x08 => 2,
                0x40 => 4,
                0x80 => 8,
                _ => 0,
            };
            self.append(idx, out, dx + ox, dy + oy, depth + 1)?;
            if flags & 0x20 == 0 {
                return Some(());
            }
        }
    }

    /// Glifo semplice a partire da `p0` (intestazione); punti e fine contorni aggiunti in coda a `out`.
    fn simple(&self, p0: usize, nc: usize, out: &mut Glyph, dx: i32, dy: i32) -> Option<()> {
        let d = self.d;
        if nc == 0 || nc > MAX_CONTOURS || out.nc + nc > MAX_CONTOURS {
            return None;
        }
        let base = out.n;
        let mut ends = [0usize; MAX_CONTOURS];
        for i in 0..nc {
            ends[i] = u16_at(d, p0 + 10 + 2 * i)? as usize;
        }
        let n = ends[nc - 1] + 1;
        if base + n > MAX_PTS {
            return None;
        }
        for i in 0..nc {
            out.ends[out.nc + i] = base + ends[i];
        }
        out.nc += nc;
        out.n = base + n;
        let mut p = p0 + 12 + 2 * nc + u16_at(d, p0 + 10 + 2 * nc)? as usize;
        let mut flags = [0u8; MAX_PTS];
        let mut k = 0;
        while k < n {
            let f = *d.get(p)?;
            p += 1;
            flags[k] = f;
            k += 1;
            if f & 8 != 0 {
                let r = *d.get(p)?;
                p += 1;
                for _ in 0..r {
                    if k < n {
                        flags[k] = f;
                        k += 1;
                    }
                }
            }
        }
        // Coordinate: bit 1 (x) / 2 (y) = byte corto; bit 4 (x) / 5 (y) = segno o valore uguale.
        let mut v = 0i32;
        for k in 0..n {
            let f = flags[k];
            if f & 2 != 0 {
                let dd = *d.get(p)? as i32;
                p += 1;
                v += if f & 16 != 0 { dd } else { -dd };
            } else if f & 16 == 0 {
                v += i16_at(d, p)? as i32;
                p += 2;
            }
            out.pts[base + k] = Pt { x: v + dx, y: 0, on: f & 1 != 0 };
        }
        v = 0;
        for k in 0..n {
            let f = flags[k];
            if f & 4 != 0 {
                let dd = *d.get(p)? as i32;
                p += 1;
                v += if f & 32 != 0 { dd } else { -dd };
            } else if f & 32 == 0 {
                v += i16_at(d, p)? as i32;
                p += 2;
            }
            out.pts[base + k].y = v + dy;
        }
        Some(())
    }
}
