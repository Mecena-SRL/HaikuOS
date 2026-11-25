//! Round-trip scrittura -> lettore FAT32. Esegui con `cargo test --features fat32-write`.
#![cfg(feature = "fat32-write")]
use haiku_alloc_sim::fat32::*;
use haiku_alloc_sim::fat32_write::*;

const TOT: usize = 66658; // settori: ~65600 cluster, FAT32

fn n83(s: &str) -> [u8; 11] {
    let mut b = [b' '; 11];
    let (base, ext) = s.split_once('.').unwrap_or((s, ""));
    b[..base.len()].copy_from_slice(base.as_bytes());
    b[8..8 + ext.len()].copy_from_slice(ext.as_bytes());
    b
}
fn blank() -> Vec<u8> {
    let mut v = vec![0u8; TOT * 512];
    format(&mut v).unwrap();
    v
}
fn find<'a>(l: &'a [DirEntry], n: &str) -> &'a DirEntry {
    l.iter().find(|e| e.name == n).expect("voce assente")
}

#[test]
fn formato_vuoto_si_legge() {
    let v = blank();
    let fs = Fat32::open(&v).unwrap();
    assert!(fs.read_dir(2).unwrap().is_empty());
}

#[test]
fn round_trip_due_file() {
    let mut v = blank();
    let big: Vec<u8> = (0..1300u32).map(|i| (i % 251) as u8).collect();
    create_file(&mut v, &n83("HELLO.TXT"), b"ciao, mondo").unwrap();
    create_file(&mut v, &n83("BIG.DAT"), &big).unwrap();
    let fs = Fat32::open(&v).unwrap();
    let root = fs.read_dir(2).unwrap();
    assert_eq!(root.len(), 2);
    assert_eq!(fs.read_file(find(&root, "HELLO.TXT")).unwrap(), b"ciao, mondo");
    assert_eq!(fs.read_file(find(&root, "BIG.DAT")).unwrap(), big);
}

#[test]
fn round_trip_file_vuoto() {
    let mut v = blank();
    create_file(&mut v, &n83("EMPTY.DAT"), &[]).unwrap();
    let fs = Fat32::open(&v).unwrap();
    let root = fs.read_dir(2).unwrap();
    assert_eq!(fs.read_file(find(&root, "EMPTY.DAT")).unwrap(), Vec::<u8>::new());
}

#[test]
fn copie_della_fat_uguali() {
    let mut v = blank();
    create_file(&mut v, &n83("A.BIN"), &[7u8; 2000]).unwrap();
    let fatsz = u32::from_le_bytes(v[36..40].try_into().unwrap()) as usize;
    let (a, b) = (32 * 512, (32 + fatsz) * 512);
    assert_eq!(v[a..a + fatsz * 512], v[b..b + fatsz * 512]);
}

#[test]
fn nome_duplicato_rifiutato() {
    let mut v = blank();
    create_file(&mut v, &n83("X.TXT"), b"1").unwrap();
    assert_eq!(create_file(&mut v, &n83("X.TXT"), b"2"), Err(FatError::Exists));
}

#[test]
fn radice_piena_rifiutata() {
    let mut v = blank();
    for i in 0..16 {
        create_file(&mut v, &n83(&format!("F{i:02}.T")), b"x").unwrap();
    }
    assert_eq!(create_file(&mut v, &n83("LAST.T"), b"x"), Err(FatError::DirFull));
}

#[test]
fn spazio_insufficiente_rifiutato_senza_modificare() {
    let mut v = blank();
    let prima = v.clone();
    let troppo = vec![1u8; 66000 * 512];
    assert_eq!(create_file(&mut v, &n83("HUGE.DAT"), &troppo), Err(FatError::NoSpace));
    assert!(v == prima, "un errore non deve lasciare tracce");
}
