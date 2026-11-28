//! Chrome di finestra generato da parametri: barra titolo + 3 pulsanti a cerchio.
//! Nessun bitmap salvato: ogni frame (o solo quando i parametri cambiano) si
//! ricalcola dai parametri. `no_std`, nessuna allocazione.

use crate::gui::{Canvas, Pixel};

/// Parametri che descrivono il chrome. Piccolo (poche decine di byte): è tutto ciò che si salva.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Chrome {
    /// Larghezza della barra in pixel.
    pub w: usize,
    /// Altezza della barra in pixel.
    pub bar: usize,
    /// Finestra attiva: pulsanti colorati invece che grigi.
    pub focused: bool,
    /// Colore di fondo della barra (ARGB).
    pub bg: u32,
}

pub const CLOSE: u32 = 0xFFE53E3E;
pub const MIN: u32 = 0xFFE0B040;
pub const MAX: u32 = 0xFF3A7D44;
pub const IDLE: u32 = 0xFF808080;

impl Chrome {
    /// Raggio dei pulsanti: un quarto della barra.
    pub fn radius(&self) -> usize {
        (self.bar / 4).max(1)
    }
}

/// Disegna la barra in `(x, y)` sul canvas. Ritaglia automaticamente ai bordi.
pub fn draw<P: Pixel>(c: &mut Canvas<P>, x: usize, y: usize, ch: &Chrome) {
    c.rect(x, y, ch.w, ch.bar, ch.bg);
    let r = ch.radius();
    let cy = y + ch.bar / 2;
    let colors = if ch.focused { [CLOSE, MIN, MAX] } else { [IDLE; 3] };
    for (i, col) in colors.iter().enumerate() {
        let cx = x + ch.bar / 2 + i * ch.bar;
        circle(c, cx, cy, r, *col);
    }
}

/// Cerchio pieno per test di distanza intera (niente floating point).
fn circle<P: Pixel>(c: &mut Canvas<P>, cx: usize, cy: usize, r: usize, col: u32) {
    let p = P::from_argb(col);
    let r2 = (r * r) as isize;
    let ri = r as isize;
    for dy in -ri..=ri {
        let yy = cy as isize + dy;
        if yy < 0 || yy as usize >= c.h {
            continue;
        }
        for dx in -ri..=ri {
            let xx = cx as isize + dx;
            if xx < 0 || xx as usize >= c.w || dx * dx + dy * dy > r2 {
                continue;
            }
            c.px[yy as usize * c.w + xx as usize] = p;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CH: Chrome = Chrome { w: 320, bar: 32, focused: true, bg: 0xFF202020 };

    #[test]
    fn draws_bar_and_buttons() {
        let mut buf = vec![0u32; 320 * 32];
        draw(&mut Canvas { px: &mut buf, w: 320, h: 32 }, 0, 0, &CH);
        assert_eq!(buf[0], CH.bg);
        assert_eq!(buf[16 * 320 + 8], CLOSE); // centro primo pulsante
        assert_eq!(buf[16 * 320 + 8 + 32], MIN);
        assert_eq!(buf[16 * 320 + 8 + 64], MAX);
    }

    #[test]
    fn clips_at_edges() {
        let mut buf = vec![0u32; 40 * 20];
        draw(&mut Canvas { px: &mut buf, w: 40, h: 20 }, 30, 0, &CH);
        assert_eq!(buf.len(), 800); // nessun panic fuori bordo
    }

    #[test]
    #[ignore]
    fn bench_generated_vs_stored() {
        use std::{hint::black_box, time::Instant};
        const N: u32 = 2000;
        for &(w, h) in &[(1920usize, 1080usize), (3840, 2160)] {
            let ch = Chrome { w, bar: 32, focused: true, bg: 0xFF202020 };
            let mut screen = vec![0u32; w * h];

            // Versione "stored": bitmap pre-renderizzato della sola barra.
            let mut stored = vec![0u32; w * ch.bar];
            draw(&mut Canvas { px: &mut stored, w, h: ch.bar }, 0, 0, &ch);

            let t = Instant::now();
            for _ in 0..N {
                draw(&mut Canvas { px: &mut screen, w, h }, 0, 0, black_box(&ch));
            }
            let gen_us = t.elapsed().as_secs_f64() * 1e6 / N as f64;

            let t = Instant::now();
            for _ in 0..N {
                for y in 0..ch.bar {
                    screen[y * w..(y + 1) * w].copy_from_slice(&stored[y * w..(y + 1) * w]);
                }
                black_box(&screen);
            }
            let blit_us = t.elapsed().as_secs_f64() * 1e6 / N as f64;

            // Versione "cache": nessun lavoro se i parametri non cambiano (dirty check).
            let last = black_box(ch);
            let t = Instant::now();
            for _ in 0..N {
                if black_box(last) != ch {
                    draw(&mut Canvas { px: &mut screen, w, h }, 0, 0, &ch);
                }
            }
            let cache_us = t.elapsed().as_secs_f64() * 1e6 / N as f64;

            println!(
                "{w}x{h}: generato {gen_us:.2} us/frame | stored blit {blit_us:.2} us/frame | cache (invariato) {cache_us:.4} us/frame | RAM stored {} B, generato {} B",
                stored.len() * 4,
                core::mem::size_of::<Chrome>()
            );
        }
    }
}
