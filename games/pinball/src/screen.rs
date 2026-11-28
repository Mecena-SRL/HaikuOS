//! Host grafico (solo std): titolo, pausa e fotogramma PPM. Il core non conosce menu né schermate.

use pinball::{Pinball, BUMPERS, FP, TABLE_H, TABLE_W};

/// Scala: 1 unità di tavolo = `S` px.
pub const S: i64 = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Title,
    Play,
    Pause,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Next {
    Go(Screen),
    Quit,
    Stay,
}

/// Titolo: invio avvia (la pallina è già in `Ready`, la lancia la leva). Pausa: `r` o invio riprende, `q` esce.
pub fn on_menu_key(s: Screen, key: &str) -> Next {
    match (s, key) {
        (Screen::Title | Screen::Pause, "q") => Next::Quit,
        (Screen::Title, _) => Next::Go(Screen::Play),
        (Screen::Pause, "r" | "") => Next::Go(Screen::Play),
        _ => Next::Stay,
    }
}

pub struct Img {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[u8; 3]>,
}

impl Img {
    fn new(w: usize, h: usize, c: [u8; 3]) -> Self {
        Img {
            w,
            h,
            px: vec![c; w * h],
        }
    }

    fn rect(&mut self, x: i64, y: i64, w: i64, h: i64, c: [u8; 3]) {
        for yy in y.max(0)..(y + h).min(self.h as i64) {
            for xx in x.max(0)..(x + w).min(self.w as i64) {
                self.px[yy as usize * self.w + xx as usize] = c;
            }
        }
    }

    fn disc(&mut self, cx: i64, cy: i64, r: i64, c: [u8; 3]) {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    self.rect(cx + dx, cy + dy, 1, 1, c);
                }
            }
        }
    }

    fn seg(&mut self, a: (i64, i64), b: (i64, i64), r: i64, c: [u8; 3]) {
        let n = (b.0 - a.0).abs().max((b.1 - a.1).abs()).max(1);
        for k in 0..=n {
            self.disc(a.0 + (b.0 - a.0) * k / n, a.1 + (b.1 - a.1) * k / n, r, c);
        }
    }

    /// Scurisce il fotogramma (fondo del menu).
    fn dim(&mut self) {
        for p in &mut self.px {
            *p = p.map(|c| c / 3);
        }
    }

    pub fn write_ppm(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        write!(out, "P6\n{} {}\n255\n", self.w, self.h)?;
        for p in &self.px {
            out.write_all(p)?;
        }
        Ok(())
    }
}

/// Fotogramma del tavolo (coordinate FP -> px) con il menu sovrapposto se richiesto.
pub fn frame(g: &Pinball, screen: Screen) -> Img {
    let (w, h) = (TABLE_W * S, TABLE_H * S);
    let px = |v: (i64, i64)| (v.0 * S, v.1 * S);
    let mut im = Img::new(w as usize, h as usize, [10, 14, 30]);
    for c in BUMPERS {
        let (x, y) = px(c);
        im.disc(x, y, 8 * S, [255, 90, 160]);
    }
    for f in &g.flips {
        let (a, t) = (f.pivot, f.tip());
        let f2 = |v: (i64, i64)| (v.0 * S / FP, v.1 * S / FP);
        im.seg(f2(a), f2(t), 2 * S, [240, 240, 250]);
    }
    for b in g.balls.iter().filter(|b| b.live) {
        im.disc(b.pos.0 * S / FP, b.pos.1 * S / FP, 3 * S, [255, 230, 80]);
    }
    if screen != Screen::Play {
        im.dim();
        let (px0, py0, pw, ph) = (w / 8, h / 4, w * 3 / 4, h / 3);
        im.rect(px0, py0, pw, ph, [24, 28, 40]);
        im.rect(px0, py0, pw, 6, [240, 200, 60]);
        let by = py0 + ph * 2 / 3;
        if screen == Screen::Title {
            im.rect(px0 + pw / 4, by, pw / 2, ph / 8, [80, 200, 110]);
        } else {
            im.rect(px0 + pw / 8, by, pw * 3 / 8, ph / 8, [80, 200, 110]);
            im.rect(px0 + pw * 9 / 16, by, pw * 5 / 16, ph / 8, [220, 70, 70]);
        }
    }
    im
}

/// Testo dei menu per il terminale.
pub fn menu_text(s: Screen) {
    match s {
        Screen::Title => println!("PINBALL\nINVIO per iniziare (leva: invio), q per uscire"),
        Screen::Pause => println!("PAUSA: r riprendi, q esci"),
        Screen::Play => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titolo_pausa_e_tasti() {
        assert_eq!(on_menu_key(Screen::Title, ""), Next::Go(Screen::Play));
        assert_eq!(on_menu_key(Screen::Title, "q"), Next::Quit);
        assert_eq!(on_menu_key(Screen::Pause, "r"), Next::Go(Screen::Play));
        assert_eq!(on_menu_key(Screen::Pause, "x"), Next::Stay);
    }

    #[test]
    fn fotogramma_ha_dimensioni_del_tavolo() {
        let g = Pinball::new();
        let im = frame(&g, Screen::Pause);
        assert_eq!(
            (im.w, im.h),
            ((TABLE_W * S) as usize, (TABLE_H * S) as usize)
        );
        assert_eq!(im.px.len(), im.w * im.h);
    }
}
