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
fn put(v: &mut [u8], o: usize, b: &[u8]) {
    v[o..o + b.len()].copy_from_slice(b);
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

/// Offset della prima voce della radice (cluster 2) nel volume formattato da `format`.
fn root_off() -> usize {
    (32 + 2 * 513) * 512
}

#[test]
fn check_pulito_dopo_scritture_normali() {
    let mut v = blank();
    create_file(&mut v, &n83("A.TXT"), &[1u8; 1500]).unwrap();
    let d = create_dir(&mut v, 2, &n83("DOCS")).unwrap();
    create_file_in(&mut v, d, &n83("B.TXT"), b"hi").unwrap();
    assert!(check(&v).unwrap().is_clean());
}

#[test]
fn cluster_persi_rilevati_e_liberati() {
    let mut v = blank();
    create_file(&mut v, &n83("A.TXT"), b"ok").unwrap();
    // Simula un crash: FAT allocata per 2 cluster, nessuna voce di directory.
    let g = (32usize, 513usize);
    for c in [100usize, 101] {
        let val: u32 = if c == 100 { 101 } else { 0x0FFF_FFFF };
        for base in [g.0, g.0 + g.1] {
            put(&mut v, base * 512 + c * 4, &val.to_le_bytes());
        }
    }
    let rep = check(&v).unwrap();
    assert_eq!(rep.lost, vec![100, 101]);
    assert_eq!(reclaim_lost(&mut v).unwrap(), 2);
    assert!(check(&v).unwrap().is_clean());
}

#[test]
fn cluster_condiviso_rilevato_e_non_riparato() {
    let mut v = blank();
    create_file(&mut v, &n83("A.TXT"), &[1u8; 600]).unwrap(); // cluster 3,4
    create_file(&mut v, &n83("B.TXT"), b"b").unwrap(); // cluster 5
    // Fa puntare B al cluster 3 di A (campo basso del cluster in offset 26 della voce 1).
    put(&mut v, root_off() + 32 + 26, &3u16.to_le_bytes());
    let rep = check(&v).unwrap();
    assert!(rep.cross_linked.contains(&3));
    assert_eq!(reclaim_lost(&mut v), Err(FatError::CrossLinked));
}

#[test]
fn sottodirectory_leggibile_e_scrivibile() {
    let mut v = blank();
    let d = create_dir(&mut v, 2, &n83("DOCS")).unwrap();
    create_file_in(&mut v, d, &n83("NOTE.TXT"), b"appunti").unwrap();
    let fs = Fat32::open(&v).unwrap();
    let root = fs.read_dir(2).unwrap();
    let docs = find(&root, "DOCS");
    assert!(docs.is_dir);
    let inside = fs.read_dir(docs.first_cluster).unwrap();
    assert_eq!(inside.len(), 1, "'.' e '..' non compaiono");
    assert_eq!(fs.read_file(find(&inside, "NOTE.TXT")).unwrap(), b"appunti");
}

#[test]
fn sottodirectory_nidificata() {
    let mut v = blank();
    let a = create_dir(&mut v, 2, &n83("A")).unwrap();
    let b = create_dir(&mut v, a, &n83("B")).unwrap();
    create_file_in(&mut v, b, &n83("C.TXT"), b"fondo").unwrap();
    let fs = Fat32::open(&v).unwrap();
    let root = fs.read_dir(2).unwrap();
    let la = fs.read_dir(find(&root, "A").first_cluster).unwrap();
    let lb = fs.read_dir(find(&la, "B").first_cluster).unwrap();
    assert_eq!(fs.read_file(find(&lb, "C.TXT")).unwrap(), b"fondo");
    assert!(check(&v).unwrap().is_clean());
}
