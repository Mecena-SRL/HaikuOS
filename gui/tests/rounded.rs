// Angoli arrotondati (solo x86): il pixel d'angolo prende il fondo T0, il bordo resta T3.
mod common;
use common::Buf;
use haiku_gui::layout::*;
use haiku_gui::render::{draw, Canvas};
use haiku_gui::{Icon, Key, Shell, Tok};

struct Round(Buf);
impl Canvas for Round {
    fn fill(&mut self, r: Rect, t: Tok) {
        self.0.fill(r, t)
    }
    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok) {
        self.0.text(x, y, s, t)
    }
    fn rounded(&self) -> bool {
        true
    }
}

static ICONS: [Icon; 1] = [Icon { label: "FILE", app: 1, title: "FILE", w: 60, h: 50 }];

#[test]
fn window_corners_are_cut_and_edges_stay() {
    let mut s = Shell::new(&ICONS, 240, 160);
    s.key(Key::A);
    let f = s.wm.top().unwrap().frame;
    let mut b = Round(Buf::new(240, 160));
    draw(&mut b, &s);
    assert_eq!(b.0.at(f.x, f.y), Tok::T0, "angolo smussato");
    assert_eq!(b.0.at(f.x + 6, f.y), Tok::T3, "bordo superiore");
    assert_eq!(b.0.at(f.x, f.y + 6), Tok::T3, "bordo sinistro");
}
