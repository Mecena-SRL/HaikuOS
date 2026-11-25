//! Test host: `cd gba && rustc --edition 2021 --test host/test_editor.rs -o /tmp/ed_t && /tmp/ed_t`
#![allow(dead_code)]
#[path = "../src/glyph.rs"]
mod glyph;
#[path = "../src/editor.rs"]
mod editor;

use editor::{Editor, MAX};

#[test]
fn carica_e_aggiunge() {
    let mut e = Editor::new();
    e.load(b"HI");
    assert_eq!(e.text(), b"HI");
    e.up(); // ' ' -> 'A'
    assert_eq!(e.pending(), b'A');
    assert!(e.commit());
    assert_eq!(e.text(), b"HIA");
}

#[test]
fn su_giu_fanno_il_giro() {
    let mut e = Editor::new();
    assert_eq!(e.pending(), b' ');
    e.down();
    assert_eq!(e.pending(), b'9');
    e.up();
    assert_eq!(e.pending(), b' ');
}

#[test]
fn cancella_e_limiti() {
    let mut e = Editor::new();
    assert!(!e.backspace());
    e.load(&[b'X'; MAX + 5]);
    assert_eq!(e.text().len(), MAX);
    assert!(!e.commit());
    assert!(e.backspace());
    assert_eq!(e.text().len(), MAX - 1);
}
