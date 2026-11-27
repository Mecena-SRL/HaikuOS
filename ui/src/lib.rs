//! Identità visiva e desktop di HaikuOS. `no_std`, senza allocazione.
//! Tutto è disegnato da forme in proporzione alla risoluzione (`unit`):
//! la stessa scena vale da 240x160 a 3840x2160 senza immagini da scalare.

#![cfg_attr(not(test), no_std)]

use haiku_editor::gui::{Canvas, Pixel};

// Palette: inchiostro, cielo, muschio, sole, argilla, carta.
pub const INK: u32 = 0xFF14213D;
pub const SKY: u32 = 0xFF3D5A80;
pub const MOSS: u32 = 0xFF81B29A;
pub const SUN: u32 = 0xFFF2CC8F;
pub const CLAY: u32 = 0xFFE07A5F;
pub const PAPER: u32 = 0xFFF4EFE6;

/// Unità base: 1/120 del lato minore, almeno 1 pixel.
pub fn unit(w: usize, h: usize) -> usize {
    (w.min(h) / 120).max(1)
}

/// Miscela due colori ARGB, `t` da 0 (solo `a`) a 255 (solo `b`).
pub fn mix(a: u32, b: u32, t: u32) -> u32 {
    let ch = |s: u32| {
        let (x, y) = ((a >> s) & 0xFF, (b >> s) & 0xFF);
        (x * (255 - t) + y * t) / 255
    };
    0xFF00_0000 | (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

fn set<P: Pixel>(c: &mut Canvas<P>, x: usize, y: usize, col: u32) {
    if x < c.w && y < c.h {
        c.px[y * c.w + x] = P::from_argb(col);
    }
}

/// Rettangolo con angoli arrotondati di raggio `r` (0 = angoli vivi).
pub fn round_rect<P: Pixel>(c: &mut Canvas<P>, x: usize, y: usize, w: usize, h: usize, r: usize, col: u32) {
    let r = r.min(w / 2).min(h / 2);
    let r2 = (r * r) as isize;
    for yy in y..y + h {
        for xx in x..x + w {
            let cx = xx.clamp(x + r, (x + w - r - 1).max(x + r));
            let cy = yy.clamp(y + r, (y + h - r - 1).max(y + r));
            let (dx, dy) = (xx as isize - cx as isize, yy as isize - cy as isize);
            if dx * dx + dy * dy <= r2 {
                set(c, xx, yy, col);
            }
        }
    }
}

pub fn circle<P: Pixel>(c: &mut Canvas<P>, cx: usize, cy: usize, r: usize, col: u32) {
    let r2 = (r * r) as isize;
    for y in cy.saturating_sub(r)..=cy + r {
        for x in cx.saturating_sub(r)..=cx + r {
            let (dx, dy) = (x as isize - cx as isize, y as isize - cy as isize);
            if dx * dx + dy * dy <= r2 {
                set(c, x, y, col);
            }
        }
    }
}

/// Collina a onda triangolare: `base` è la linea di terra, `amp` l'altezza massima.
fn hill<P: Pixel>(c: &mut Canvas<P>, base: usize, amp: usize, period: usize, col: u32) {
    let period = period.max(2);
    for x in 0..c.w {
        let t = x % period;
        let tri = if t * 2 < period { t * 2 } else { (period - t) * 2 };
        let top = base.saturating_sub(amp * tri / period);
        for y in top..c.h {
            set(c, x, y, col);
        }
    }
}

/// Sfondo: cielo che scende nell'inchiostro, sole e tre colline.
pub fn wallpaper<P: Pixel>(c: &mut Canvas<P>) {
    let (w, h) = (c.w, c.h);
    for y in 0..h {
        let col = mix(SKY, INK, (y * 255 / h.max(1)) as u32);
        c.px[y * w..(y + 1) * w].iter_mut().for_each(|p| *p = P::from_argb(col));
    }
    circle(c, w * 3 / 4, h * 2 / 5, w.min(h) / 6, SUN);
    hill(c, h * 55 / 100, h / 10, w / 2, mix(MOSS, SKY, 90));
    hill(c, h * 70 / 100, h / 12, w / 3, MOSS);
    hill(c, h * 86 / 100, h / 16, w * 2 / 5, mix(INK, MOSS, 40));
}

/// Icone e logo su griglia 16x16, scalate sulla dimensione `s` in pixel.
struct Grid {
    x: usize,
    y: usize,
    s: usize,
}

impl Grid {
    fn rr<P: Pixel>(&self, c: &mut Canvas<P>, gx: usize, gy: usize, gw: usize, gh: usize, r: usize, col: u32) {
        let s = self.s;
        round_rect(c, self.x + gx * s / 16, self.y + gy * s / 16, gw * s / 16, gh * s / 16, r * s / 16, col);
    }

    fn dot<P: Pixel>(&self, c: &mut Canvas<P>, cx: usize, cy: usize, r: usize, col: u32) {
        let s = self.s;
        circle(c, self.x + cx * s / 16, self.y + cy * s / 16, r * s / 16, col);
    }
}

/// Le otto applicazioni del desktop.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Terminal,
    Calculator,
    Files,
    Editor,
    Settings,
    Clock,
    Browser,
    Reader,
}

pub const ICONS: [Icon; 8] = [
    Icon::Terminal,
    Icon::Calculator,
    Icon::Files,
    Icon::Editor,
    Icon::Settings,
    Icon::Clock,
    Icon::Browser,
    Icon::Reader,
];

/// Icona quadrata `size` x `size` con piastra di carta e disegno a colori piatti.
pub fn icon<P: Pixel>(c: &mut Canvas<P>, which: Icon, x: usize, y: usize, size: usize) {
    let g = Grid { x, y, s: size };
    g.rr(c, 0, 0, 16, 16, 4, PAPER);
    match which {
        Icon::Terminal => {
            g.rr(c, 2, 3, 12, 10, 2, INK);
            g.rr(c, 4, 6, 2, 2, 0, MOSS);
            g.rr(c, 6, 8, 2, 2, 0, MOSS);
            g.rr(c, 4, 10, 2, 2, 0, MOSS);
            g.rr(c, 9, 10, 4, 2, 0, SUN);
        }
        Icon::Calculator => {
            g.rr(c, 3, 2, 10, 12, 2, SKY);
            g.rr(c, 5, 4, 6, 3, 1, PAPER);
            for row in [8, 11] {
                for col in [5, 8, 11] {
                    g.rr(c, col, row, 2, 2, 0, PAPER);
                }
            }
        }
        Icon::Files => {
            g.rr(c, 2, 4, 6, 3, 1, CLAY);
            g.rr(c, 2, 6, 12, 8, 2, SUN);
        }
        Icon::Editor => {
            g.rr(c, 4, 2, 8, 12, 1, SKY);
            g.rr(c, 6, 5, 4, 1, 0, PAPER);
            g.rr(c, 6, 8, 4, 1, 0, PAPER);
            g.rr(c, 6, 11, 2, 1, 0, CLAY);
        }
        Icon::Settings => {
            g.rr(c, 7, 1, 2, 3, 0, SKY);
            g.rr(c, 7, 12, 2, 3, 0, SKY);
            g.rr(c, 1, 7, 3, 2, 0, SKY);
            g.rr(c, 12, 7, 3, 2, 0, SKY);
            g.dot(c, 8, 8, 5, SKY);
            g.dot(c, 8, 8, 2, PAPER);
        }
        Icon::Clock => {
            g.dot(c, 8, 8, 6, SUN);
            g.rr(c, 7, 4, 2, 5, 0, INK);
            g.rr(c, 7, 7, 4, 2, 0, INK);
        }
        Icon::Browser => {
            g.dot(c, 8, 8, 6, MOSS);
            g.rr(c, 7, 2, 2, 12, 0, PAPER);
            g.rr(c, 2, 7, 12, 2, 0, PAPER);
        }
        Icon::Reader => {
            g.rr(c, 3, 2, 10, 12, 1, CLAY);
            g.rr(c, 5, 5, 6, 1, 0, PAPER);
            g.rr(c, 5, 8, 6, 1, 0, PAPER);
            g.rr(c, 5, 11, 4, 1, 0, PAPER);
        }
    }
}

/// Logo HaikuOS: tre righe di lunghezza 5-7-5, come un haiku.
pub fn logo<P: Pixel>(c: &mut Canvas<P>, x: usize, y: usize, size: usize) {
    let g = Grid { x, y, s: size };
    g.rr(c, 0, 0, 16, 16, 4, SKY);
    g.rr(c, 3, 4, 10, 2, 0, CLAY);
    g.rr(c, 1, 7, 14, 2, 0, SUN);
    g.rr(c, 3, 10, 10, 2, 0, MOSS);
}

/// Desktop: sfondo, griglia di icone a sinistra, barra in basso con il logo.
pub fn desktop<P: Pixel>(c: &mut Canvas<P>) {
    let (w, h) = (c.w, c.h);
    let u = unit(w, h);
    let (bar, size, step) = (12 * u, 10 * u, 14 * u);
    wallpaper(c);
    c.rect(0, h.saturating_sub(bar), w, bar, INK);
    logo(c, u, h.saturating_sub(bar) + u, size);
    let cols = ((w.saturating_sub(8 * u)) / step).max(1);
    for (i, which) in ICONS.iter().enumerate() {
        let (col, row) = (i % cols, i / cols);
        let (x, y) = (4 * u + col * step, 4 * u + row * step);
        if y + size <= h.saturating_sub(bar) && x + size <= w {
            icon(c, *which, x, y, size);
        }
    }
}

/// Schermata di avvio: logo al centro e barra di caricamento.
/// `progress` va da 0 a 1000 (millesimi).
pub fn splash<P: Pixel>(c: &mut Canvas<P>, progress: u32) {
    let (w, h) = (c.w, c.h);
    let u = unit(w, h);
    c.clear(INK);
    let s = (w.min(h) / 3).max(16);
    let (lx, ly) = (w.saturating_sub(s) / 2, h.saturating_sub(s) / 2);
    logo(c, lx, ly, s);
    let (bw, by) = (w / 3, ly + s + 8 * u);
    let fill = bw * progress.min(1000) as usize / 1000;
    c.rect((w - bw) / 2, by, bw, 2 * u, SKY);
    c.rect((w - bw) / 2, by, fill, 2 * u, SUN);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas(buf: &mut [u32], w: usize, h: usize) -> Canvas<'_, u32> {
        Canvas { px: buf, w, h }
    }

    #[test]
    fn unit_scales_with_screen() {
        assert_eq!(unit(240, 160), 1);
        assert_eq!(unit(640, 480), 4);
        assert_eq!(unit(1920, 1080), 9);
        assert_eq!(unit(3840, 2160), 18);
    }

    #[test]
    fn desktop_renders_at_any_size() {
        for (w, h) in [(240, 160), (640, 480), (800, 600), (1366, 768), (1920, 1080), (3840, 2160)] {
            let mut buf = vec![0u32; w * h];
            desktop(&mut canvas(&mut buf, w, h));
            // La barra in basso è inchiostro, il resto non è vuoto.
            assert_eq!(buf[(h - 1) * w + w / 2], INK);
            assert_ne!(buf[0], 0);
        }
    }

    #[test]
    fn splash_progress_fills_bar() {
        let (w, h) = (1920, 1080);
        let mut buf = vec![0u32; w * h];
        let mut c = canvas(&mut buf, w, h);
        splash(&mut c, 500);
        let u = unit(w, h);
        let by = (h - (w.min(h) / 3).max(16)) / 2 + (w.min(h) / 3).max(16) + 8 * u;
        let bw = w / 3;
        let x0 = (w - bw) / 2;
        assert_eq!(buf[by * w + x0], SUN);
        assert_eq!(buf[by * w + x0 + bw - 1], SKY);
    }

    #[test]
    fn mix_endpoints() {
        assert_eq!(mix(SKY, INK, 0), SKY);
        assert_eq!(mix(SKY, INK, 255), INK);
    }
}
