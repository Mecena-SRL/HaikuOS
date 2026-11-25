//! Test host: `cd gba && rustc --edition 2021 --test host/test_fontmgr.rs -o /tmp/fm_t && /tmp/fm_t`
#![allow(dead_code)]
#[path = "../src/glyph.rs"]
mod glyph;
#[path = "../src/fontmgr.rs"]
mod fontmgr;

use fontmgr::FontMgr;

#[test]
fn installa_cerca_e_limiti_di_nome() {
    let mut m = FontMgr::new();
    assert_eq!(m.install(b"Sistema"), Ok(0));
    assert_eq!(m.install(b"Sistema"), Err(()));
    assert_eq!(m.install(b""), Err(()));
    assert_eq!(m.install(b"nomequalunghissimo"), Err(()));
    assert_eq!(m.find(b"Sistema"), Some(0));
    assert_eq!(m.name(0), Some(&b"Sistema"[..]));
}

#[test]
fn registro_pieno_e_attivi_limitati() {
    let mut m = FontMgr::new();
    for n in [&b"A"[..], b"B", b"C", b"D"] {
        m.install(n).unwrap();
    }
    assert_eq!(m.install(b"E"), Err(()));
    for i in 0..4u8 {
        m.activate(i).unwrap();
    }
    assert_eq!(m.active_count(), 4);
    m.deactivate(3).unwrap();
    assert!(m.activate(3).is_ok());
    assert_eq!(m.activate(9), Err(()));
}

#[test]
fn sistema_non_disattivabile() {
    let mut m = FontMgr::new();
    m.install(b"Sistema").unwrap();
    m.install(b"Gioco").unwrap();
    m.set_system(0).unwrap();
    assert!(m.is_active(0));
    assert_eq!(m.deactivate(0), Err(()));
    m.activate(1).unwrap();
    assert!(m.deactivate(1).is_ok());
    assert_eq!(m.system(), Some(0));
}

#[test]
fn anteprima_disegna_nel_framebuffer() {
    let m = FontMgr::new();
    let mut fb = [0u16; 16 * 8];
    m.preview(&mut fb, 16, 8, 0, 0, b"A", 0x7FFF);
    assert!(fb.iter().any(|&p| p == 0x7FFF));
    assert_eq!(std::mem::size_of::<FontMgr>() <= 128, true);
}
