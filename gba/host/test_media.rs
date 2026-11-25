//! Test host delle app media (scacchi e lettore sulla superficie 64×48).
//! `cd gba && rustc --edition 2021 --test host/test_media.rs -o /tmp/claude/media_t && /tmp/claude/media_t`
#![allow(dead_code)]
#[path = "../src/glyph.rs"]
mod glyph;
#[path = "../src/chess.rs"]
mod chess;
#[path = "../src/media_ui.rs"]
mod media_ui;

use media_ui::{ChessView, Key, Reader, ROWS, TEXT, W, H, WHITE_PC, CURSOR, BG};
use std::io::Write;

fn count(px: &[u16], c: u16) -> usize {
    px.iter().filter(|&&p| p == c).count()
}

fn square_px(px: &[u16], file: usize, rank: usize, c: u16) -> usize {
    let (x, y) = (file * 6, (7 - rank) * 6);
    let mut n = 0;
    for yy in y..y + 6 {
        for xx in x..x + 6 {
            if px[yy * W + xx] == c {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn scacchi_posizione_iniziale_disegnata() {
    let v = ChessView::new();
    let mut px = vec![0u16; W * H];
    v.draw(&mut px);
    // e2 (file 4, traversa 1) ha un pedone bianco: pixel bianchi nella casa.
    assert!(square_px(&px, 4, 1, WHITE_PC) > 0);
    // e4 è vuota: nessun pezzo.
    assert_eq!(square_px(&px, 4, 3, WHITE_PC), 0);
    // Cursore su e2: sottolineatura in fondo alla casa.
    assert_eq!(px[(6 * 6 + 5) * W + 4 * 6], CURSOR); // traversa 2 (riga 6) in basso
}

#[test]
fn scacchi_tastierino_muove_un_pedone() {
    let mut v = ChessView::new();
    // cursore su e2 (12): A seleziona, su due volte per e4, A muove.
    v.key(Key::A);
    assert_eq!(v.from, Some(12));
    v.key(Key::Up);
    v.key(Key::Up);
    v.key(Key::A);
    assert_eq!(v.board.at(28), chess::piece(chess::PAWN, chess::WHITE));
    assert_eq!(v.board.turn, chess::BLACK);
    assert_eq!(v.from, None);
}

#[test]
fn scacchi_mossa_non_legale_non_cambia_la_scacchiera() {
    let mut v = ChessView::new();
    v.key(Key::A); // seleziona e2
    v.key(Key::Up);
    v.key(Key::Up);
    v.key(Key::Up); // cursore su e5: e2e5 non è legale
    let prima = v.board;
    v.key(Key::A);
    assert_eq!(v.board, prima);
    assert_eq!(v.board.turn, chess::WHITE);
    assert_eq!(v.from, None);
}

#[test]
fn scacchi_b_annulla_selezione() {
    let mut v = ChessView::new();
    v.key(Key::A);
    v.key(Key::B);
    assert_eq!(v.from, None);
}

#[test]
fn lettore_pagine_e_a_capo() {
    let mut r = Reader::new(b"ciao mondo di prova in piu righe per la pagina uno e ancora altre parole per riempire il testo oltre sei righe da quindici colonne");
    assert!(r.pages() >= 2);
    assert!(!r.prev());
    assert!(r.next());
    assert_eq!(r.page(), 1);
    while r.next() {}
    assert_eq!(r.page(), r.pages() - 1);
}

#[test]
fn lettore_disegna_testo_e_indicatore() {
    let r = Reader::new(b"I");
    let mut px = vec![0u16; W * H];
    r.draw(&mut px);
    // Prima riga, prima colonna di "I": pixel di testo alla posizione di origine.
    assert_eq!(px[2 * W + 2], TEXT);
    // Una pagina sola: indicatore "1 OF 1" presente.
    assert!(count(&px, TEXT) > 0);
    assert_eq!(px[0], BG);
    assert_eq!(ROWS, 6);
}

/// Scrive i frame in PPM (P6) per lo screenshot. Attivo solo con SHOT_DIR.
#[test]
fn schermate_per_lo_screenshot() {
    let dir = match std::env::var("SHOT_DIR") {
        Ok(d) => d,
        Err(_) => return,
    };
    let c5 = |v: u16| ((v & 31) * 255 / 31) as u8;
    let save = |name: &str, px: &[u16]| {
        let mut f = std::fs::File::create(format!("{dir}/{name}.ppm")).unwrap();
        write!(f, "P6\n{} {}\n255\n", W, H).unwrap();
        for &p in px {
            f.write_all(&[c5(p), c5(p >> 5), c5(p >> 10)]).unwrap();
        }
    };
    let mut v = ChessView::new();
    v.key(Key::A);
    v.key(Key::Up);
    v.key(Key::Up);
    v.key(Key::A); // 1. e4
    let mut px = vec![0u16; W * H];
    v.draw(&mut px);
    save("chess", &px);
    let r = Reader::new(b"Il mare e' vasto. Nelle pagine di un ebook ogni riga va a capo a una parola intera, come in un libro vero.");
    let mut px = vec![0u16; W * H];
    r.draw(&mut px);
    save("reader", &px);
}
