// Canvas di prova: un token per pixel, più l'elenco dei testi disegnati.
#![allow(dead_code)]
use haiku_gui::layout::Rect;
use haiku_gui::render::Canvas;
use haiku_gui::Tok;

pub struct Buf {
    pub w: i32,
    pub h: i32,
    pub px: Vec<Tok>,
    pub texts: Vec<String>,
}

impl Buf {
    pub fn new(w: i32, h: i32) -> Buf {
        Buf { w, h, px: vec![Tok::T0; (w * h) as usize], texts: Vec::new() }
    }
    pub fn at(&self, x: i32, y: i32) -> Tok {
        self.px[(y * self.w + x) as usize]
    }
}

impl Canvas for Buf {
    fn fill(&mut self, r: Rect, t: Tok) {
        for y in r.y.max(0)..(r.y + r.h).min(self.h) {
            for x in r.x.max(0)..(r.x + r.w).min(self.w) {
                self.px[(y * self.w + x) as usize] = t;
            }
        }
    }
    fn text(&mut self, _x: i32, _y: i32, s: &str, _t: Tok) {
        self.texts.push(s.to_string());
    }
}
