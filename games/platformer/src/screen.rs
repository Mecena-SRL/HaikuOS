//! Host grafico (solo std): titolo e pausa come overlay ARGB disegnati con `haiku-gui::Canvas`.

use haiku_gui::layout::{Rect, MENU_ROW};
use haiku_gui::palette::Tok;
use haiku_gui::render::Canvas;
use platformer::{VIEW_H, VIEW_W};

#[path = "../../../gba/src/font.rs"]
mod font;

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

/// Titolo: invio avvia. Pausa: `r` o invio riprende, `q` esce.
pub fn on_menu_key(s: Screen, key: &str) -> Next {
    match (s, key) {
        (Screen::Title | Screen::Pause, "q") => Next::Quit,
        (Screen::Title, _) => Next::Go(Screen::Play),
        (Screen::Pause, "r" | "") => Next::Go(Screen::Play),
        _ => Next::Stay,
    }
}

/// Framebuffer ARGB con il trait `Canvas` di gui: rettangoli pieni e testo 3x5 di gba.
pub struct Fb<'a>(pub &'a mut [u32]);

fn argb(t: Tok) -> u32 {
    0xFF00_0000 | t.rgb()
}

impl Canvas for Fb<'_> {
    fn fill(&mut self, r: Rect, t: Tok) {
        let c = argb(t);
        for y in r.y.max(0)..(r.y + r.h).min(VIEW_H) {
            for x in r.x.max(0)..(r.x + r.w).min(VIEW_W) {
                self.0[(y * VIEW_W + x) as usize] = c;
            }
        }
    }

    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok) {
        for (i, ch) in s.bytes().enumerate() {
            let gx = x + i as i32 * font::ADVANCE as i32;
            for (row, bits) in font::glyph(ch).iter().enumerate() {
                for col in 0..font::GLYPH_W {
                    if (bits >> (font::GLYPH_W - 1 - col)) & 1 == 1 {
                        self.fill(Rect::new(gx + col as i32, y + row as i32, 1, 1), t);
                    }
                }
            }
        }
    }
}

/// Sovrappone il menu: fondo scurito, pannello con titolo e voci (la prima selezionata).
/// Le voci sono centrate nella riga: `Canvas::menu` di gui mette il testo fuori riga.
pub fn overlay(fb: &mut [u32], s: Screen) {
    if s == Screen::Play {
        return;
    }
    for p in fb.iter_mut() {
        *p = 0xFF00_0000 | ((*p >> 2) & 0x003F_3F3F);
    }
    let (title, rows): (&str, &[&str]) = match s {
        Screen::Title => ("PLATFORMER", &["INIZIA", "ESCI"]),
        _ => ("PAUSA", &["RIPRENDI", "ESCI"]),
    };
    let mut c = Fb(fb);
    let p = Rect::new(VIEW_W / 8, VIEW_H / 4, VIEW_W * 3 / 4, 22 + MENU_ROW * rows.len() as i32 + 4);
    c.fill(p, Tok::T0);
    c.fill(Rect::new(p.x, p.y, p.w, 3), Tok::A(1));
    c.text(p.x + 6, p.y + 6, title, Tok::A(1));
    for (i, label) in rows.iter().enumerate() {
        let y = p.y + 18 + i as i32 * MENU_ROW;
        let (bg, fg) = if i == 0 { (Tok::T2, Tok::T0) } else { (Tok::T0, Tok::T2) };
        c.fill(Rect::new(p.x + 4, y, p.w - 8, MENU_ROW), bg);
        c.text(p.x + 8, y + 2, label, fg);
    }
}

/// Testo dei menu per il terminale.
pub fn menu_text(s: Screen) {
    match s {
        Screen::Title => println!("PLATFORMER\nINVIO per iniziare, q per uscire"),
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
}
