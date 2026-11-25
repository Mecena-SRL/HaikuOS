//! Rasterizzazione di testo da contorni OpenType (solo host): riempimento non-zero,
//! curve quadratiche appiattite. Vedi `docs/font-manager.md`.

use crate::opentype::{self, Metrics, OtError};

const CURVE_STEPS: usize = 8;

/// Canvas in bit: una cella per pixel, riga per riga.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub px: Vec<bool>,
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Self {
        Canvas { w, h, px: vec![false; w * h] }
    }

    /// Una riga di testo per riga di pixel: `#` acceso, spazio spento.
    pub fn to_art(&self) -> Vec<String> {
        (0..self.h)
            .map(|r| (0..self.w).map(|c| if self.px[r * self.w + c] { '#' } else { ' ' }).collect())
            .collect()
    }
}

/// Appiattisce un contorno (x, y, su_curva) in un poligono; le curve quadratiche
/// diventano segmenti. Punti fuori curva consecutivi hanno un punto implicito a metà.
fn flatten(c: &[(f64, f64, bool)]) -> Vec<(f64, f64)> {
    let n = c.len();
    let mut e: Vec<(f64, f64, bool)> = Vec::with_capacity(2 * n);
    for i in 0..n {
        let a = c[i];
        let b = c[(i + 1) % n];
        e.push(a);
        if !a.2 && !b.2 {
            e.push(((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0, true));
        }
    }
    let Some(s) = e.iter().position(|p| p.2) else { return Vec::new() };
    let len = e.len();
    let mut out = vec![(e[s].0, e[s].1)];
    let mut k = 1;
    while k <= len {
        let p = e[(s + k) % len];
        if p.2 {
            out.push((p.0, p.1));
            k += 1;
        } else {
            let q = e[(s + k + 1) % len];
            let (x0, y0) = *out.last().unwrap();
            for t in 1..=CURVE_STEPS {
                let u = t as f64 / CURVE_STEPS as f64;
                let v = 1.0 - u;
                out.push((
                    v * v * x0 + 2.0 * v * u * p.0 + u * u * q.0,
                    v * v * y0 + 2.0 * v * u * p.1 + u * u * q.1,
                ));
            }
            k += 2;
        }
    }
    out
}

/// Attraversamento di un lato alla riga `y`: x e direzione (+1 / -1).
fn cross(a: (f64, f64), b: (f64, f64), y: f64) -> Option<(f64, i32)> {
    let x = a.0 + (y - a.1) * (b.0 - a.0) / (b.1 - a.1);
    if a.1 <= y && y < b.1 {
        Some((x, 1))
    } else if b.1 <= y && y < a.1 {
        Some((x, -1))
    } else {
        None
    }
}

/// Riempie i poligoni nel canvas con regola non-zero.
fn fill(c: &mut Canvas, polys: &[Vec<(f64, f64)>]) {
    for row in 0..c.h {
        let y = row as f64 + 0.5;
        let mut xs: Vec<(f64, i32)> = Vec::new();
        for p in polys {
            for i in 0..p.len() {
                if let Some(x) = cross(p[i], p[(i + 1) % p.len()], y) {
                    xs.push(x);
                }
            }
        }
        xs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut wind = 0;
        for k in 0..xs.len().saturating_sub(1) {
            wind += xs[k].1;
            if wind != 0 {
                let (x0, x1) = (xs[k].0, xs[k + 1].0);
                for col in 0..c.w {
                    let cx = col as f64 + 0.5;
                    if cx >= x0 && cx < x1 {
                        c.px[row * c.w + col] = true;
                    }
                }
            }
        }
    }
}

/// Disegna una riga di testo a `px_em` pixel per em. Il canvas è largo quanto l'avanzamento
/// totale e alto quanto il font (yMax - yMin). Glifi mancanti non disegnano nulla.
pub fn render_text(data: &[u8], text: &str, px_em: f64) -> Result<Canvas, OtError> {
    let Metrics { units_per_em, y_min, y_max } = opentype::metrics(data)?;
    let scale = px_em / units_per_em as f64;
    let mut glyphs = Vec::new();
    for ch in text.chars() {
        let gid = opentype::glyph_id(data, ch)?;
        glyphs.push((gid, opentype::advance(data, gid)?));
    }
    let total: u32 = glyphs.iter().map(|g| g.1 as u32).sum();
    let w = (total as f64 * scale).ceil() as usize;
    let h = ((y_max - y_min) as f64 * scale).ceil() as usize;
    let mut canvas = Canvas::new(w, h);
    let mut pen = 0i32;
    for (gid, adv) in glyphs {
        if let Some(o) = opentype::outline(data, gid)? {
            let polys: Vec<Vec<(f64, f64)>> = o
                .contours
                .iter()
                .map(|c| {
                    let pts: Vec<(f64, f64, bool)> = c
                        .iter()
                        .map(|&(x, y, on)| (((x + pen) as f64) * scale, ((y_max - y) as f64) * scale, on))
                        .collect();
                    flatten(&pts)
                })
                .collect();
            fill(&mut canvas, &polys);
        }
        pen += adv as i32;
    }
    Ok(canvas)
}

#[cfg(test)]
mod tests {
    use super::flatten;

    /// Angolo (10,10) smussato da una curva quadratica: area attesa 100 - 50/3 ≈ 83.3.
    #[test]
    fn curva_quadratica_smussa_lo_spigolo() {
        let c = [(0.0, 0.0, true), (10.0, 0.0, true), (10.0, 10.0, false), (0.0, 10.0, true)];
        let p = flatten(&c);
        let n = p.len();
        let area: f64 = (0..n)
            .map(|i| p[i].0 * p[(i + 1) % n].1 - p[(i + 1) % n].0 * p[i].1)
            .sum::<f64>()
            / 2.0;
        assert!((area - 83.33).abs() < 0.6, "area {area}"); // approssimazione a 8 passi
    }
}
