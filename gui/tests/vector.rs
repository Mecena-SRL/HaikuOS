// Primitive vettoriali di base: rettangoli arrotondati e linee.
mod common;
use common::Buf;
use haiku_gui::layout::Rect;
use haiku_gui::render::Canvas;
use haiku_gui::Tok;

#[test]
fn linea_orizzontale() {
    let mut b = Buf::new(10, 5);
    b.line(1, 2, 8, 2, 1, Tok::T1);
    for x in 1..=8 {
        assert_eq!(b.at(x, 2), Tok::T1);
    }
    assert_eq!(b.at(0, 2), Tok::T0);
    assert_eq!(b.at(9, 2), Tok::T0);
}

#[test]
fn linea_verticale() {
    let mut b = Buf::new(5, 10);
    b.line(2, 1, 2, 8, 1, Tok::T1);
    for y in 1..=8 {
        assert_eq!(b.at(2, y), Tok::T1);
    }
    assert_eq!(b.at(2, 0), Tok::T0);
}

#[test]
fn linea_diagonale_collega_gli_estremi() {
    let mut b = Buf::new(10, 10);
    b.line(0, 0, 9, 9, 1, Tok::T2);
    assert_eq!(b.at(0, 0), Tok::T2);
    assert_eq!(b.at(4, 4), Tok::T2);
    assert_eq!(b.at(9, 9), Tok::T2);
    assert_eq!(b.at(9, 0), Tok::T0);
}

#[test]
fn linea_spessa_e_piena() {
    let mut b = Buf::new(10, 10);
    b.line(2, 5, 7, 5, 3, Tok::T3);
    for y in 4..=6 {
        assert_eq!(b.at(4, y), Tok::T3);
    }
    assert_eq!(b.at(4, 3), Tok::T0);
}

#[test]
fn rect_rounded_di_base_riempie_il_rettangolo() {
    let mut b = Buf::new(8, 8);
    b.rect_rounded(Rect::new(1, 1, 4, 4), 2, Tok::T1);
    assert_eq!(b.at(1, 1), Tok::T1);
    assert_eq!(b.at(4, 4), Tok::T1);
    assert_eq!(b.at(5, 5), Tok::T0);
}
