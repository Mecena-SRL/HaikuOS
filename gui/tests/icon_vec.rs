// Icone vettoriali generate da icons/gen_vec.py: ogni icona ha rettangoli validi.
mod common;
use common::Buf;
use haiku_gui::icons::{draw_vec, vec_rects, ICON_VEC};
use haiku_gui::layout::Rect;
use haiku_gui::Tok;

#[test]
fn every_icon_has_rects() {
    assert_eq!(ICON_VEC.len(), 60);
    for (name, rects) in ICON_VEC {
        assert!(!rects.is_empty(), "{name}");
        for &(x, y, w, h) in rects.iter() {
            assert!(w > 0 && h > 0 && x >= 0 && y >= 0 && x + w <= 16 && y + h <= 16, "{name}");
        }
    }
}

#[test]
fn known_and_unknown_names() {
    assert!(vec_rects("clock").is_some());
    assert!(vec_rects("settings").is_some());
    assert!(vec_rects("nope").is_none());
}

#[test]
fn draw_scales_into_cell() {
    let mut b = Buf::new(64, 64);
    assert!(draw_vec(&mut b, Rect::new(0, 0, 64, 64), "clock", Tok::T1));
    let on = (0..64).flat_map(|y| (0..64).map(move |x| (x, y))).filter(|&(x, y)| b.at(x, y) == Tok::T1).count();
    assert!(on > 0);
    assert!(!draw_vec(&mut b, Rect::new(0, 0, 64, 64), "nope", Tok::T1));
}
