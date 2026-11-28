// Pixel attesi: barra, sfondo, finestra attiva e inattiva.
mod common;
use common::Buf;
use haiku_gui::layout::*;
use haiku_gui::render::draw;
use haiku_gui::{Icon, Key, Shell, Tok};

static ICONS: [Icon; 2] = [
    Icon { label: "FILE", app: 1, title: "FILE", w: 60, h: 50 },
    Icon { label: "TERM", app: 2, title: "TERM", w: 40, h: 30 },
];

#[test]
fn bar_background_icons_and_window_colors() {
    let mut s = Shell::new(&ICONS, 240, 160);
    let mut b = Buf::new(240, 160);
    draw(&mut b, &s);
    assert_eq!(b.at(0, 0), Tok::T3);
    assert_eq!(b.at(0, BAR_H), Tok::T0);
    assert_eq!(b.at(GRID_X, GRID_Y), Tok::T1); // icona 0 selezionata
    assert_eq!(b.at(GRID_X + PITCH_X + 1, GRID_Y + 1), Tok::T2); // icona 1 normale: blocco
    assert_eq!(b.at(GRID_X + PITCH_X + 3, GRID_Y + 4), Tok::T0); // segno del glifo TERM

    s.key(Key::A);
    let f = s.wm.top().unwrap().frame;
    let mut b = Buf::new(240, 160);
    draw(&mut b, &s);
    assert_eq!(b.at(f.x + f.w / 2, f.y + f.h - 1), Tok::T1, "nessun bordo: il corpo arriva al bordo");
    assert_eq!(b.at(f.x + 2, f.y + 2), Tok::T2, "titolo attivo");
    assert_eq!(b.at(cb(f), cb_y(f)), Tok::T0, "pulsante chiudi");
    let c = content(f);
    assert_eq!(b.at(c.x, c.y), Tok::T1, "corpo");
    assert!(b.texts.iter().any(|t| t == "FILE"));
}

#[test]
fn inactive_title_is_t1() {
    let mut s = Shell::new(&ICONS, 240, 160);
    s.key(Key::A);
    s.key(Key::Start);
    s.key(Key::Right);
    s.key(Key::A);
    // Ora la finestra 2 è in cima; la 1 è sotto e ha titolo T1.
    let f1 = s.wm.get(0).unwrap().frame;
    let mut b = Buf::new(240, 160);
    draw(&mut b, &s);
    assert_eq!(b.at(f1.x + 2, f1.y + 2), Tok::T1);
}

fn cb(f: Rect) -> i32 { close_button(f).x }
fn cb_y(f: Rect) -> i32 { close_button(f).y }
