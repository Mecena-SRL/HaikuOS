//! Rasterizzatore vettoriale RGB: poligoni e cerchi antialiasati, testo vettoriale di `gui`.
//! Ogni forma è una lista di vertici in coordinate reali: la scena scala a qualunque risoluzione.

use haiku_gui::layout::Rect;
use haiku_gui::palette::Tok;
use haiku_gui::render::Canvas;
use haiku_gui::vfont;
use std::io::{self, Write};

pub type Rgb = [u8; 3];

/// Sotto-scanline per pixel: copertura verticale a 4 livelli.
const SUB: usize = 4;
/// Corpo del testo di default (il rendering di gioco usa `vfont` direttamente con la sua taglia).
const TEXT_PX: i32 = 14;

pub struct Frame {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Rgb>,
}

impl Frame {
    pub fn new(w: usize, h: usize) -> Self {
        Frame { w, h, px: vec![[0, 0, 0]; w * h] }
    }

    /// Mescola `c` sul pixel con copertura `a` (0..=1).
    fn mix(&mut self, x: usize, y: usize, c: Rgb, a: f32) {
        let p = &mut self.px[y * self.w + x];
        for k in 0..3 {
            p[k] = (p[k] as f32 + (c[k] as f32 - p[k] as f32) * a + 0.5) as u8;
        }
    }

    /// Frame riempito col colore di sfondo, senza passate in più.
    pub fn with_fill(w: usize, h: usize, c: Rgb) -> Self {
        Frame { w, h, px: vec![c; w * h] }
    }

    /// Sfumatura verticale tra `top` (riga `y0`) e `bottom` (riga `y1`, esclusa).
    pub fn vgrad(&mut self, y0: usize, y1: usize, top: Rgb, bottom: Rgb) {
        let span = (y1.saturating_sub(y0)).max(1) as f32;
        for y in y0..y1.min(self.h) {
            let t = (y - y0) as f32 / span;
            let c: Rgb = core::array::from_fn(|k| (top[k] as f32 + (bottom[k] as f32 - top[k] as f32) * t) as u8);
            self.px[y * self.w..(y + 1) * self.w].fill(c);
        }
    }

    /// Velo di colore su tutto il frame, con opacità `a`.
    pub fn tint(&mut self, c: Rgb, a: f32) {
        for p in &mut self.px {
            *p = core::array::from_fn(|k| (p[k] as f32 + (c[k] as f32 - p[k] as f32) * a + 0.5) as u8);
        }
    }

    /// Poligono pieno (regola pari), vertici in ordine, con antialiasing.
    pub fn poly(&mut self, pts: &[(f32, f32)], c: Rgb) {
        self.poly_a(pts, c, 1.0);
    }

    /// Come `poly`, con opacità `a` (0..=1).
    pub fn poly_a(&mut self, pts: &[(f32, f32)], c: Rgb, a: f32) {
        if pts.len() < 3 {
            return;
        }
        let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for &(x, y) in pts {
            x0 = x0.min(x);
            x1 = x1.max(x);
            y0 = y0.min(y);
            y1 = y1.max(y);
        }
        let xa = x0.floor().max(0.0) as usize;
        let xb = (x1.ceil().max(0.0) as usize).min(self.w);
        let ya = y0.floor().max(0.0) as usize;
        let yb = (y1.ceil().max(0.0) as usize).min(self.h);
        if xa >= xb || ya >= yb {
            return;
        }
        let mut cov = vec![0f32; xb - xa];
        let mut xs: Vec<f32> = Vec::with_capacity(pts.len());
        for y in ya..yb {
            cov.fill(0.0);
            for s in 0..SUB {
                let sy = y as f32 + (s as f32 + 0.5) / SUB as f32;
                xs.clear();
                for i in 0..pts.len() {
                    let (ax, ay) = pts[i];
                    let (bx, by) = pts[(i + 1) % pts.len()];
                    if (ay <= sy) != (by <= sy) {
                        xs.push(ax + (sy - ay) * (bx - ax) / (by - ay));
                    }
                }
                xs.sort_by(f32::total_cmp);
                for pair in xs.chunks_exact(2) {
                    let a = pair[0].max(xa as f32);
                    let b = pair[1].min(xb as f32);
                    if a >= b {
                        continue;
                    }
                    for px in a.floor() as usize..b.ceil() as usize {
                        let lo = a.max(px as f32);
                        let hi = b.min(px as f32 + 1.0);
                        cov[px - xa] += (hi - lo) / SUB as f32;
                    }
                }
            }
            for (i, &cv) in cov.iter().enumerate() {
                if cv > 0.0 {
                    self.mix(xa + i, y, c, (cv * a).min(1.0));
                }
            }
        }
    }

    /// Rettangolo allineato, come poligono.
    pub fn quad(&mut self, x: f32, y: f32, w: f32, h: f32, c: Rgb) {
        self.poly(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)], c);
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, c: Rgb) {
        self.disc(cx, cy, r, c, 1.0);
    }

    pub fn disc(&mut self, cx: f32, cy: f32, r: f32, c: Rgb, a: f32) {
        let pts: Vec<(f32, f32)> = (0..32)
            .map(|i| {
                let a = i as f32 * std::f32::consts::TAU / 32.0;
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect();
        self.poly_a(&pts, c, a);
    }

    /// Segmento di spessore `w`, come rettangolo ruotato.
    pub fn line(&mut self, a: (f32, f32), b: (f32, f32), w: f32, c: Rgb) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let (nx, ny) = (-dy / len * w / 2.0, dx / len * w / 2.0);
        self.poly(
            &[
                (a.0 + nx, a.1 + ny),
                (b.0 + nx, b.1 + ny),
                (b.0 - nx, b.1 - ny),
                (a.0 - nx, a.1 - ny),
            ],
            c,
        );
    }

    /// Scurisce tutto il fotogramma (sfondo dei menu).
    pub fn dim(&mut self) {
        for p in &mut self.px {
            *p = p.map(|c| c / 3);
        }
    }

    pub fn write_ppm(&self, out: &mut impl Write) -> io::Result<()> {
        write!(out, "P6\n{} {}\n255\n", self.w, self.h)?;
        for p in &self.px {
            out.write_all(p)?;
        }
        Ok(())
    }
}

pub fn rgb(t: Tok) -> Rgb {
    let c = t.rgb();
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

impl Canvas for Frame {
    fn fill(&mut self, r: Rect, t: Tok) {
        let c = rgb(t);
        for y in r.y.max(0)..(r.y + r.h).min(self.h as i32) {
            for x in r.x.max(0)..(r.x + r.w).min(self.w as i32) {
                self.px[y as usize * self.w + x as usize] = c;
            }
        }
    }

    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok) {
        vfont::draw_text(self, x, y, TEXT_PX, s, t);
    }

    fn blend(&mut self, x: i32, y: i32, a: u8, t: Tok) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.mix(x as usize, y as usize, rgb(t), a as f32 / 255.0);
        }
    }

    fn smooth(&self) -> bool {
        true
    }
}
