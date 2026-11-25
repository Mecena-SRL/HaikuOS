//! Adattatore GBA per il sistema grafico comune (`haiku-gui`, gui/).
//! Implementa `Canvas` sopra il framebuffer 240×160 a 15 bit e il font 3×5.
//! Non ancora collegato a `main.rs`: il desktop attuale usa ancora `desktop.rs`.
#![allow(dead_code)]
use crate::font;
use haiku_gui::layout::Rect;
use haiku_gui::render::Canvas;
use haiku_gui::{Shell, Tok};

pub struct Fb<'a> {
    pub px: &'a mut [u16],
    pub w: usize,
    pub h: usize,
}

impl<'a> Canvas for Fb<'a> {
    fn fill(&mut self, r: Rect, t: Tok) {
        let c = t.gba();
        let x0 = r.x.max(0) as usize;
        let y0 = r.y.max(0) as usize;
        let x1 = ((r.x + r.w).max(0) as usize).min(self.w);
        let y1 = ((r.y + r.h).max(0) as usize).min(self.h);
        for y in y0..y1 {
            for x in x0..x1 {
                self.px[y * self.w + x] = c;
            }
        }
    }

    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok) {
        if x < 0 || y < 0 {
            return;
        }
        font::draw(self.px, self.w, self.h, x as usize, y as usize, s.as_bytes(), t.gba());
    }
}

/// Disegna la scrivania sul framebuffer.
pub fn paint(px: &mut [u16], s: &Shell) {
    let mut fb = Fb { px, w: s.w as usize, h: s.h as usize };
    haiku_gui::render::draw(&mut fb, s);
}
