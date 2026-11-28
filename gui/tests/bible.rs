// Regole della bibbia grafica: token, misure, etichette.
use haiku_gui::layout::*;
use haiku_gui::{Icon, Tok};

#[test]
fn gba_and_rgb_describe_the_same_colors() {
    for t in [Tok::T0, Tok::T1, Tok::T2, Tok::T3] {
        let g = t.gba();
        assert!(g < 1 << 15, "15 bit");
        let ex = |v: u16| ((v << 3) | (v >> 2)) as u32;
        let (r, gg, b) = (ex(g & 31), ex((g >> 5) & 31), ex((g >> 10) & 31));
        assert_eq!((r << 16) | (gg << 8) | b, t.rgb(), "{:?}", t);
    }
}

#[test]
fn measures_match_bible() {
    assert_eq!((BAR_H, TITLE_H, CLOSE, ICON), (12, 8, 8, 16));
    assert_eq!((GRID_X, GRID_Y, PITCH_X, PITCH_Y, COLS), (8, 24, 48, 40, 4));
    assert_eq!(icon_cell(5), Rect::new(8 + 48, 24 + 40, 16, 16));
    let f = Rect::new(10, 20, 100, 60);
    assert_eq!(close_button(f), Rect::new(10 + 100 - 1 - 8, 20, 8, 8));
}

#[test]
fn labels_are_uppercase_for_3x5_font() {
    let icons = [Icon { label: "FILE", app: 0, title: "", w: 1, h: 1 }];
    assert!(icons.iter().all(|i| !i.label.bytes().any(|b| b.is_ascii_lowercase())));
}
