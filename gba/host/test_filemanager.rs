//! Test host: `cd gba && rustc --edition 2021 --test host/test_filemanager.rs -o /tmp/fm_t && /tmp/fm_t`
#![allow(dead_code)]
#[path = "../src/fs.rs"]
mod fs;
#[path = "../src/filemanager.rs"]
mod filemanager;

use filemanager::{FileManager, ROWS};
use fs::Fs;

fn store() -> Fs {
    let mut f = Fs::new();
    f.mkdir(b"docs", 1).unwrap();
    f.create(b"readme", 1).unwrap();
    f.create(b"docs/a", 1).unwrap();
    f
}

#[test]
fn radice_elenca_solo_i_figli_diretti() {
    let f = store();
    let mut m = FileManager::new();
    m.refresh(&f);
    assert_eq!(m.visible().len(), 2); // docs, readme; docs/a no
}

#[test]
fn radice_vuota() {
    let f = Fs::new();
    let mut m = FileManager::new();
    m.refresh(&f);
    assert!(m.visible().is_empty());
    assert_eq!(m.enter(&f), None);
}

#[test]
fn entra_e_torna_indietro() {
    let f = store();
    let mut m = FileManager::new();
    m.refresh(&f);
    assert_eq!(m.enter(&f), None); // "docs" è la prima voce
    assert_eq!(m.dir(), b"docs");
    assert_eq!(m.visible().len(), 1); // docs/a
    m.back(&f);
    assert_eq!(m.dir(), b"");
    m.back(&f); // alla radice non cambia nulla
    assert_eq!(m.dir(), b"");
}

#[test]
fn file_ritorna_id_e_non_cambia_dir() {
    let f = store();
    let mut m = FileManager::new();
    m.refresh(&f);
    m.down(); // "readme"
    let id = m.enter(&f).expect("un file va aperto");
    assert_eq!(f.info(id).unwrap().0, b"readme");
    assert_eq!(m.dir(), b"");
}

#[test]
fn cursore_non_esce_dai_limiti() {
    let f = store();
    let mut m = FileManager::new();
    m.refresh(&f);
    m.up();
    assert_eq!(m.cursor(), 0);
    for _ in 0..10 {
        m.down();
    }
    assert_eq!(m.cursor(), 1);
}

#[test]
fn scorre_per_tenere_visibile_il_cursore() {
    let mut f = Fs::new();
    for i in 0..12u8 {
        let name = [b'f', b'0' + i / 10, b'0' + i % 10];
        f.create(&name, 1).unwrap();
    }
    let mut m = FileManager::new();
    m.refresh(&f);
    assert_eq!(m.visible().len(), ROWS);
    for _ in 0..11 {
        m.down();
    }
    assert_eq!(m.cursor(), 11);
    let v = m.visible();
    assert_eq!(v.len(), ROWS);
    assert_eq!(v.last().copied(), Some(f.find_name(b"f11").unwrap()));
}

#[test]
fn file_rimosso_aggiorna_la_lista() {
    let mut f = store();
    let mut m = FileManager::new();
    m.refresh(&f);
    let id = f.find_name(b"readme").unwrap();
    f.remove(id, 1).unwrap();
    m.refresh(&f);
    assert_eq!(m.visible().len(), 1);
}
