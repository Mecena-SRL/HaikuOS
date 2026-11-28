//! Host grafico (solo std): il tavolo, titolo, pausa e HUD passano dal trait `Canvas` di `haiku-gui`.
//! Il disegno è in coordinate del tavolo (1 px = 1 unità); il PPM è ingrandito di `SCALE`.

use haiku_gui::layout::{Rect, MENU_ROW};
use haiku_gui::palette::Tok;
use haiku_gui::render::Canvas;
use pinball::{Pinball, BUMPERS, FP, TABLE_H, TABLE_W};

#[path = "../../../gba/src/font.rs"]
mod font;

/// Ingrandimento del PPM di uscita.
pub const SCALE: usize = 3;
/// Altezza dell'em del testo di sistema, in unità del tavolo.
const TEXT_EM: i32 = 8;

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

/// Fotogramma RGB del tavolo, più piccolo dell'uscita di `SCALE`.
pub struct Img {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[u8; 3]>,
}

fn rgb(t: Tok) -> [u8; 3] {
    let c = t.rgb();
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

impl Img {
    fn put(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 3]) {
        for yy in y.max(0)..(y + h).min(self.h as i32) {
            for xx in x.max(0)..(x + w).min(self.w as i32) {
                self.px[yy as usize * self.w + xx as usize] = c;
            }
        }
    }

    fn disc(&mut self, cx: i32, cy: i32, r: i32, c: [u8; 3]) {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    self.put(cx + dx, cy + dy, 1, 1, c);
                }
            }
        }
    }

    fn seg(&mut self, a: (i32, i32), b: (i32, i32), r: i32, c: [u8; 3]) {
        let n = (b.0 - a.0).abs().max((b.1 - a.1).abs()).max(1);
        for k in 0..=n {
            self.disc(a.0 + (b.0 - a.0) * k / n, a.1 + (b.1 - a.1) * k / n, r, c);
        }
    }

    /// Scurisce il fotogramma (fondo dei menu).
    fn dim(&mut self) {
        for p in &mut self.px {
            *p = p.map(|c| c / 3);
        }
    }

    /// PPM P6 ingrandito di `SCALE` (vicino più prossimo).
    pub fn write_ppm(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        let (w, h) = (self.w * SCALE, self.h * SCALE);
        write!(out, "P6\n{w} {h}\n255\n")?;
        for y in 0..h {
            for x in 0..w {
                out.write_all(&self.px[(y / SCALE) * self.w + x / SCALE])?;
            }
        }
        Ok(())
    }
}

impl Canvas for Img {
    fn fill(&mut self, r: Rect, t: Tok) {
        self.put(r.x, r.y, r.w, r.h, rgb(t));
    }

    /// Testo di sistema (Inter, `fonts/README.md`) al posto del 3×5 di gba.
    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok) {
        haiku_gui::vfont::draw_text(self, x, y, TEXT_EM, s, t);
    }

    /// Miscela con copertura `a` (antialiasing del testo di sistema).
    fn blend(&mut self, x: i32, y: i32, a: u8, t: Tok) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        let c = rgb(t);
        let p = &mut self.px[y as usize * self.w + x as usize];
        for k in 0..3 {
            p[k] = ((p[k] as u32 * (255 - a as u32) + c[k] as u32 * a as u32) / 255) as u8;
        }
    }
}

/// Menu sovrapposto: fondo scurito, pannello con titolo e voci, la prima voce selezionata.
/// Le voci sono disegnate qui (centrate nella riga): `Canvas::menu` di gui ha il testo fuori riga.
fn overlay(im: &mut Img, s: Screen) {
    im.dim();
    let (title, rows): (&str, &[&str]) = match s {
        Screen::Title => ("PINBALL", &["INIZIA", "ESCI"]),
        _ => ("PAUSA", &["RIPRENDI", "ESCI"]),
    };
    let p = Rect::new(12, 70, 136, 18 + MENU_ROW * rows.len() as i32 + 4);
    im.rect_rounded(p, 4, Tok::T0);
    im.text(p.x + 8, p.y + 5, title, Tok::A(1));
    for (i, label) in rows.iter().enumerate() {
        let y = p.y + 16 + i as i32 * MENU_ROW;
        let (bg, fg) = if i == 0 { (Tok::T2, Tok::T0) } else { (Tok::T0, Tok::T2) };
        im.fill(Rect::new(p.x + 4, y, p.w - 8, MENU_ROW), bg);
        im.text(p.x + 8, y + 2, label, fg);
    }
}

/// Fotogramma del tavolo (1 px = 1 unità) con HUD in gioco o menu sovrapposto.
pub fn frame(g: &Pinball, screen: Screen) -> Img {
    let (w, h) = (TABLE_W as usize, TABLE_H as usize);
    let mut im = Img { w, h, px: vec![[10, 14, 30]; w * h] };
    let to = |v: (i64, i64)| ((v.0 / FP) as i32, (v.1 / FP) as i32);
    for c in BUMPERS {
        let (x, y) = (c.0 as i32, c.1 as i32);
        im.disc(x, y, 8, [255, 90, 160]);
    }
    for f in &g.flips {
        im.seg(to(f.pivot), to(f.tip()), 2, [240, 240, 250]);
    }
    for b in g.balls.iter().filter(|b| b.live) {
        let (x, y) = to(b.pos);
        im.disc(x, y, 3, [255, 230, 80]);
    }
    match screen {
        Screen::Play => im.text(2, 2, &format!("PUNTI {}  VITE {}", g.score, g.lives), Tok::T1),
        s => overlay(&mut im, s),
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
        let im = frame(&Pinball::new(), Screen::Pause);
        assert_eq!((im.w, im.h), (TABLE_W as usize, TABLE_H as usize));
        assert_eq!(im.px.len(), im.w * im.h);
    }
}
