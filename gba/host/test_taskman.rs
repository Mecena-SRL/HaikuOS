//! Test host: `cd gba && rustc --edition 2021 --test host/test_taskman.rs -o /tmp/tm_t && /tmp/tm_t`
#![allow(dead_code)]
#[path = "../src/alloc.rs"]
mod alloc;
#[path = "../src/monolith.rs"]
mod monolith;
#[path = "../src/font.rs"]
mod font;
#[path = "../src/taskman.rs"]
mod taskman;

use monolith::{Monolith, SLOT_BYTES};
use taskman::{line, rows, State};

fn text(r: &taskman::Row) -> String {
    let mut b = [0u8; 24];
    let n = line(r, &mut b);
    String::from_utf8(b[..n].to_vec()).unwrap()
}

#[test]
fn stato_iniziale_tutto_libero() {
    let m = Monolith::new();
    for r in rows(&m) {
        assert_eq!(r.state, State::Free);
        assert_eq!(r.bytes, 0);
    }
}

#[test]
fn registrato_poi_attivo_poi_revocato() {
    let mut m = Monolith::new();
    m.register(1);
    assert_eq!(rows(&m)[1].state, State::Registered);
    m.request(1, SLOT_BYTES).expect("quota concessa");
    let r = rows(&m)[1];
    assert_eq!(r.state, State::Active);
    assert_eq!(r.bytes, SLOT_BYTES);
    m.revoke(1);
    assert_eq!(rows(&m)[1].state, State::Registered);
}

#[test]
fn riga_di_testo() {
    let mut m = Monolith::new();
    m.register(0);
    m.request(0, SLOT_BYTES).unwrap();
    assert_eq!(text(&rows(&m)[0]), "P0 ATTIVO 32K");
    assert_eq!(text(&rows(&m)[2]), "P2 LIBERO 0K");
}

#[test]
fn stato_non_cambia_gli_altri_slot() {
    let mut m = Monolith::new();
    m.register(3);
    let rs = rows(&m);
    assert_eq!(rs[3].state, State::Registered);
    assert!(rs[..3].iter().all(|r| r.state == State::Free));
}

#[test]
fn disegno_scrive_pixel() {
    let m = Monolith::new();
    let mut fb = [0u16; 64 * 40];
    taskman::draw(&mut fb, 64, 40, &m, 0, 0, 0x7FFF);
    assert!(fb.iter().any(|&p| p != 0));
}
