//! Test host: `cd gba && rustc --edition 2021 --test host/test_apps.rs -o /tmp/apps_t && /tmp/apps_t`
#![allow(dead_code)]
#[path = "../src/glyph.rs"]
mod glyph;
#[path = "../src/editor.rs"]
mod editor;
#[path = "../src/apps/notes.rs"]
mod notes;
#[path = "../src/apps/clock.rs"]
mod clock;
#[path = "../src/apps/settings.rs"]
mod settings;

use clock::Clock;
use notes::{Notes, N};
use settings::{Settings, NAME_MAX, PALETTES, THEMES};

#[test]
fn note_crea_scrive_e_seleziona() {
    let mut n = Notes::new();
    assert!(!n.commit()); // nessuna nota
    assert!(n.nuova());
    n.up(); // ' ' -> 'A'
    assert!(n.commit());
    assert_eq!(n.testo(), b"A");
    assert!(n.is_dirty());
    n.mark_saved();
    assert!(!n.is_dirty());
    assert!(n.nuova());
    assert_eq!(n.selected(), 1);
    assert_eq!(n.testo(), b"");
    assert!(n.seleziona(0));
    assert_eq!(n.testo(), b"A");
    assert!(!n.seleziona(5));
}

#[test]
fn note_limite_e_caricamento() {
    let mut n = Notes::new();
    for _ in 0..N {
        assert!(n.nuova());
    }
    assert!(!n.nuova());
    assert_eq!(n.count(), N);
    n.mark_saved();
    assert!(n.carica(2, b"ciao"));
    assert!(!n.is_dirty()); // caricare non sporca
    assert!(n.seleziona(2));
    assert_eq!(n.testo(), b"ciao");
    assert!(n.backspace());
    assert_eq!(n.testo(), b"cia");
    assert!(n.is_dirty());
}

#[test]
fn orologio_avanza_e_fa_il_giro() {
    let mut c = Clock::new(9, 5);
    assert_eq!(&c.testo(), b"09:05");
    c.tick(59);
    assert_eq!(c.hms(), (9, 5, 0));
    c.tick(1);
    assert_eq!(c.hms(), (9, 5, 1));
    c.tick(60 * 59);
    assert_eq!(c.hms(), (9, 6, 0));
    c.set(23, 59);
    c.tick(60 * 60); // 23:59:00 + 1 minuto
    assert_eq!(c.testo(), *b"00:00");
    c.tick(u32::MAX); // nessun overflow
    assert!(c.hms().0 < 24);
}

#[test]
fn impostazioni_nome_e_limiti() {
    let mut s = Settings::new();
    assert_eq!(s.nome(), b"HAIKUOS");
    for _ in 0..NAME_MAX {
        s.push(b'x');
    }
    assert!(!s.push(b'x'));
    assert_eq!(s.nome().len(), NAME_MAX);
    assert!(!s.push(0x07)); // non stampabile
    while s.pop() {}
    assert!(!s.pop());
    assert_eq!(s.nome(), b"");
}

#[test]
fn impostazioni_temi_fanno_il_giro() {
    let mut s = Settings::new();
    assert_eq!(s.palette(), PALETTES[0]);
    s.prev_theme();
    assert_eq!(s.theme(), THEMES - 1);
    s.next_theme();
    assert_eq!(s.theme(), 0);
    for _ in 0..THEMES {
        assert!(s.palette().iter().all(|&c| c < 0x8000));
        s.next_theme();
    }
    assert_eq!(s.theme(), 0);
}

#[path = "../src/apps/archive.rs"]
mod archive;

#[test]
fn archivio_contenitore() {
    let mut out = [0u8; 64];
    let n = archive::pack(&[(b"a", b"xy")], &mut out).unwrap();
    assert_eq!(n, 4 + 1 + 1 + 2 + 2);
    let mut r = archive::Reader::new(&out[..n]).unwrap();
    assert_eq!(r.next_file().unwrap().unwrap().1, b"xy");
}
