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
#[path = "../src/ascon.rs"]
mod ascon;
#[path = "../src/apps/sysutil.rs"]
mod sysutil;

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

fn env_test() -> sysutil::Env {
    sysutil::Env { free_blocks: 88, total_blocks: 128, block: 64, ram: 288 * 1024, pool: 224 * 1024 }
}

#[test]
fn utility_disco_info() {
    let env = env_test();
    let (a, b) = sysutil::disk(&env);
    assert_eq!(a.as_bytes(), b"DISCO 31%");
    assert_eq!(b.as_bytes(), b"2560/8192 B");
    let (a, b) = sysutil::info(&env);
    assert_eq!(a.as_bytes(), b"RAM 288K");
    assert_eq!(b.as_bytes(), b"POOL 224K");
}

#[test]
fn utility_stampa_coda() {
    let mut s = sysutil::Spool::new();
    for _ in 0..4 {
        assert!(s.submit());
    }
    assert!(!s.submit());
    assert_eq!(s.out().0.as_bytes(), b"CODA 4/4");
    assert!(s.finish());
    assert_eq!(s.out().1.as_bytes(), b"STAMPATE 1");
    assert_eq!(s.pending(), 3);
}

#[test]
fn utility_nota_midi() {
    assert_eq!(sysutil::hz10(69), 4400);
    assert_eq!(sysutil::hz10(60), 2616);
    assert_eq!(sysutil::hz10(0), 81);
    assert_eq!(sysutil::hz10(127), 125440);
    assert_eq!(sysutil::note(69).0.as_bytes(), b"A4 440.0 HZ");
    assert_eq!(sysutil::note(60).0.as_bytes(), b"C4 261.6 HZ");
    assert_eq!(sysutil::note(0).0.as_bytes(), b"C-1 8.1 HZ");
}

#[test]
fn utility_colore() {
    assert_eq!(sysutil::parse_rgb(b"FF8800"), Some([255, 136, 0]));
    assert_eq!(sysutil::parse_rgb(b"ff8800"), Some([255, 136, 0]));
    assert_eq!(sysutil::parse_rgb(b"FF88"), None);
    assert_eq!(sysutil::parse_rgb(b"GG0000"), None);
    assert_eq!(sysutil::colour_name([250, 10, 5]), &b"ROSSO"[..]);
    assert_eq!(sysutil::colour_name([0, 0, 0]), &b"NERO"[..]);
    assert_eq!(sysutil::colour_name([128, 128, 128]), &b"GRIGIO"[..]);
    assert_eq!(sysutil::colour_name([0, 255, 255]), &b"CIANO"[..]);
    assert_eq!(sysutil::colour([255, 0, 255]).1.as_bytes(), b"#FF00FF");
}

#[test]
fn utility_comandi() {
    let env = env_test();
    let mut sp = sysutil::Spool::new();
    assert_eq!(sysutil::command(b"  disco ", &env, &mut sp).unwrap().0.as_bytes(), b"DISCO 31%");
    assert!(sysutil::command(b"ciao", &env, &mut sp).is_none());
    assert_eq!(sysutil::command(b"STAMPA", &env, &mut sp).unwrap().0.as_bytes(), b"CODA 1/4");
    assert_eq!(sysutil::command(b"STAMPA FINE", &env, &mut sp).unwrap().1.as_bytes(), b"STAMPATE 1");
    assert!(sysutil::command(b"NOTA 300", &env, &mut sp).is_none());
    assert_eq!(sysutil::command(b"NOTA 69", &env, &mut sp).unwrap().0.as_bytes(), b"A4 440.0 HZ");
    assert_eq!(sysutil::command(b"COLORE 00FF00", &env, &mut sp).unwrap().0.as_bytes(), b"VERDE");
}

#[test]
fn utility_miniatura_e_nome() {
    let mut px = vec![0u16; sysutil::SHOT_W * sysutil::SHOT_H];
    px[0] = 0x7FFF;
    let mut out = [0u8; sysutil::THUMB_BYTES];
    assert!(sysutil::thumb(&px, &mut out));
    assert_eq!(out[0], 1);
    assert_eq!(out.iter().map(|b| b.count_ones()).sum::<u32>(), 1);
    assert!(!sysutil::thumb(&px[..10], &mut out));
    assert_eq!(sysutil::THUMB_BYTES, 4800);
    assert!(sysutil::THUMB_BYTES <= 8192);
    assert_eq!(sysutil::shot_name(7).as_bytes(), b"SHOT0007.B1");
}

#[test]
fn archivio_cifrato() {
    let key = [7u8; 16];
    let nonce = [9u8; 16];
    let files: [(&[u8], &[u8]); 1] = [(b"a.txt", b"ciao")];
    let mut arc = [0u8; 64];
    let n = archive::pack(&files, &mut arc).unwrap();
    let mut sealed = [0u8; 128];
    let m = archive::seal(&key, &nonce, &arc[..n], &mut sealed).unwrap();
    assert_eq!(m, n + archive::TAG);
    let mut plain = [0u8; 64];
    let k = archive::open(&key, &nonce, &sealed[..m], &mut plain).unwrap();
    assert_eq!(&plain[..k], &arc[..n]);
    sealed[0] ^= 1;
    assert!(archive::open(&key, &nonce, &sealed[..m], &mut plain).is_none());
}
