//! Primitive 2D intere, senza float e senza sqrt (adatte a no_std e a CPU senza FPU).
//! Portate da games/wilkanoid/src/render.rs. Copertura in Q8 (0..=256): 128 + (r² − d²)·128 / r.

use crate::Frame;

/// Font 3x5: righe a 3 bit, bit alto a sinistra. A-Z e 0-9; il resto è vuoto.
const FONT: [[u8; 5]; 26] = [
    [0b010, 0b101, 0b111, 0b101, 0b101], [0b110, 0b101, 0b110, 0b101, 0b110],
    [0b011, 0b100, 0b100, 0b100, 0b011], [0b110, 0b101, 0b101, 0b101, 0b110],
    [0b111, 0b100, 0b110, 0b100, 0b111], [0b111, 0b100, 0b110, 0b100, 0b100],
    [0b011, 0b100, 0b101, 0b101, 0b011], [0b101, 0b101, 0b111, 0b101, 0b101],
    [0b111, 0b010, 0b010, 0b010, 0b111], [0b001, 0b001, 0b001, 0b101, 0b010],
    [0b101, 0b101, 0b110, 0b101, 0b101], [0b100, 0b100, 0b100, 0b100, 0b111],
    [0b101, 0b111, 0b111, 0b101, 0b101], [0b101, 0b111, 0b111, 0b111, 0b101],
    [0b010, 0b101, 0b101, 0b101, 0b010], [0b110, 0b101, 0b110, 0b100, 0b100],
    [0b010, 0b101, 0b101, 0b111, 0b011], [0b110, 0b101, 0b110, 0b101, 0b101],
    [0b011, 0b100, 0b010, 0b001, 0b110], [0b111, 0b010, 0b010, 0b010, 0b010],
    [0b101, 0b101, 0b101, 0b101, 0b111], [0b101, 0b101, 0b101, 0b101, 0b010],
    [0b101, 0b101, 0b111, 0b111, 0b101], [0b101, 0b101, 0b010, 0b101, 0b101],
    [0b101, 0b101, 0b010, 0b010, 0b010], [0b111, 0b001, 0b010, 0b100, 0b111],
];
const DIG: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111], [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001], [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111], [0b111, 0b001, 0b010, 0b010, 0b010],
    [0b111, 0b101, 0b111, 0b101, 0b111], [0b111, 0b101, 0b111, 0b001, 0b111],
];

fn glyph(c: u8) -> [u8; 5] {
    match c {
        b'A'..=b'Z' => FONT[(c - b'A') as usize],
        b'0'..=b'9' => DIG[(c - b'0') as usize],
        _ => [0; 5],
    }
}

/// Miscela due colori ARGB con `a` in 0..=256 (0 = solo `d`).
pub fn mix(d: u32, s: u32, a: u32) -> u32 {
    let ch = |sh: u32| {
        let (sc, dc) = ((s >> sh) & 0xFF, (d >> sh) & 0xFF);
        (sc * a + dc * (256 - a)) >> 8
    };
    0xFF00_0000 | (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// Vista logica → schermo: scala intera uniforme con letterbox centrato.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport {
    pub scale: i32,
    pub ox: i32,
    pub oy: i32,
}

impl Viewport {
    /// Il campo logico `lw` x `lh` entra in `w` x `h` alla scala intera massima (almeno 1).
    pub fn fit(w: i32, h: i32, lw: i32, lh: i32) -> Self {
        let scale = (w / lw).min(h / lh).max(1);
        Self { scale, ox: (w - lw * scale) / 2, oy: (h - lh * scale) / 2 }
    }
}

impl Frame<'_> {
    /// Rettangolo arrotondato con gradiente verticale `top`→`bot` e opacità `alpha` (0..=256).
    #[allow(clippy::too_many_arguments)]
    pub fn rr_a(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, r: i32, top: u32, bot: u32, alpha: u32) {
        let r = r.clamp(0, ((x1 - x0).min(y1 - y0)) / 2);
        let (r2, h) = (r * r, (y1 - y0).max(1));
        for y in y0.max(0)..y1.min(self.h as i32) {
            let t = ((y - y0) * 256 / h) as u32;
            let col = mix(top, bot, t);
            let cy = y.clamp(y0 + r, y1 - 1 - r);
            let dy = y - cy;
            for x in x0.max(0)..x1.min(self.w as i32) {
                let cx = x.clamp(x0 + r, x1 - 1 - r);
                let d2 = (x - cx) * (x - cx) + dy * dy;
                let cov = if r == 0 { 256 } else { (128 + (r2 - d2) * 128 / r).clamp(0, 256) };
                let a = (cov as u32 * alpha) >> 8;
                if a > 0 {
                    let i = y as usize * self.w + x as usize;
                    self.px[i] = mix(self.px[i], col, a);
                }
            }
        }
    }

    /// Cerchio antialiasato di raggio `r` centrato in (cx, cy), opacità `alpha` (0..=256).
    pub fn disc_a(&mut self, cx: i32, cy: i32, r: i32, col: u32, alpha: u32) {
        let r2 = r * r;
        for y in (cy - r).max(0)..(cy + r + 1).min(self.h as i32) {
            for x in (cx - r).max(0)..(cx + r + 1).min(self.w as i32) {
                let d2 = (x - cx) * (x - cx) + (y - cy) * (y - cy);
                let cov = if r == 0 { 0 } else { (128 + (r2 - d2) * 128 / r).clamp(0, 256) };
                let a = (cov as u32 * alpha) >> 8;
                if a > 0 {
                    let i = y as usize * self.w + x as usize;
                    self.px[i] = mix(self.px[i], col, a);
                }
            }
        }
    }

    /// Testo 3x5 (A-Z, 0-9). `s` è la dimensione di un punto in pixel; avanzamento 4·s per carattere.
    pub fn text(&mut self, text: &[u8], x: i32, y: i32, s: i32, col: u32) {
        for (k, &c) in text.iter().enumerate() {
            let g = glyph(c.to_ascii_uppercase());
            let gx = x + k as i32 * 4 * s;
            for (row, bits) in g.iter().enumerate() {
                for bit in 0..3i32 {
                    if bits & (4 >> bit) != 0 {
                        self.fill_rect(gx + bit * s, y + row as i32 * s, s, s, col);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disc_is_round_and_clipped() {
        let mut buf = [0u32; 9 * 9];
        let mut f = Frame { w: 9, h: 9, px: &mut buf };
        f.disc_a(4, 4, 3, 0xFFFF_FFFF, 256);
        assert_eq!(buf[4 * 9 + 4], 0xFFFF_FFFF); // centro pieno
        assert_eq!(buf[0], 0); // angolo fuori
        let mut small = [0u32; 4];
        let mut f2 = Frame { w: 2, h: 2, px: &mut small };
        f2.disc_a(0, 0, 5, 0xFFFF_FFFF, 256); // quasi tutto fuori: non deve andare in panic
    }

    #[test]
    fn rounded_rect_cuts_corners() {
        let mut buf = [0u32; 10 * 10];
        let mut f = Frame { w: 10, h: 10, px: &mut buf };
        f.rr_a(0, 0, 10, 10, 4, 0xFFFF_FFFF, 0xFFFF_FFFF, 256);
        assert_eq!(buf[5 * 10 + 5], 0xFFFF_FFFF);
        assert_eq!(buf[0], 0); // angolo tagliato
    }

    #[test]
    fn text_draws_glyph_pixels() {
        let mut buf = [0u32; 8 * 8];
        let mut f = Frame { w: 8, h: 8, px: &mut buf };
        f.text(b"A", 0, 0, 1, 0xFFFF_FFFF);
        assert_eq!(buf.iter().filter(|&&c| c != 0).count(), 10); // A = 10 pixel
    }

    #[test]
    fn viewport_letterboxes_480x272() {
        let v = Viewport::fit(480, 272, 240, 135);
        assert_eq!(v, Viewport { scale: 2, ox: 0, oy: 1 });
    }
}
