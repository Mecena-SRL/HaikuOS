//! Test del lettore exFAT in sola lettura. Il volume di prova è costruito qui dal codice.
use haiku_alloc_sim::exfat::*;

const BPS: usize = 512;
const FAT_SECT: usize = 24;
const HEAP_SECT: usize = 32;
const TOT_SECT: usize = 48;

fn put(v: &mut [u8], o: usize, b: &[u8]) {
    v[o..o + b.len()].copy_from_slice(b);
}
fn set_fat(v: &mut [u8], c: usize, val: u32) {
    put(v, FAT_SECT * BPS + c * 4, &val.to_le_bytes());
}
fn cluster_off(c: usize) -> usize {
    (HEAP_SECT + (c - 2)) * BPS
}
fn utf16(name: &str) -> Vec<u16> {
    name.encode_utf16().collect()
}

/// Insieme di voci per un file o una cartella: file + stream + name. Checksum calcolato.
fn entry_set(name: &str, attr: u16, flags: u8, cluster: u32, size: u64, deleted: bool) -> Vec<u8> {
    let units = utf16(name);
    let name_entries = units.len().div_ceil(15);
    let n = 1 + name_entries;
    let mut s = vec![0u8; 32 * (n + 1)];
    s[0] = if deleted { 0x05 } else { 0x85 };
    s[1] = n as u8;
    s[4..6].copy_from_slice(&attr.to_le_bytes());
    s[32] = if deleted { 0x40 } else { 0xC0 };
    s[33] = flags;
    s[35] = units.len() as u8;
    s[52..56].copy_from_slice(&cluster.to_le_bytes());
    s[56..64].copy_from_slice(&size.to_le_bytes());
    for k in 0..name_entries {
        let o = 32 * (2 + k);
        s[o] = if deleted { 0x41 } else { 0xC1 };
        for j in 0..15 {
            if let Some(&u) = units.get(k * 15 + j) {
                s[o + 2 + 2 * j..o + 4 + 2 * j].copy_from_slice(&u.to_le_bytes());
            }
        }
    }
    let sum = set_checksum(&s);
    s[2..4].copy_from_slice(&sum.to_le_bytes());
    s
}

/// Volume: root (cluster 2) = "Relazione.txt" (cluster 3, catena, "ciao!"), "DOCS" (cluster 4),
/// una voce cancellata e una voce bitmap da ignorare.
/// DOCS = "A.TXT" (cluster 5, contiguo, 600 byte: 512 'a' + 88 'b').
fn volume() -> Vec<u8> {
    let mut v = vec![0u8; TOT_SECT * BPS];
    put(&mut v, 3, b"EXFAT   ");
    put(&mut v, 72, &(TOT_SECT as u64).to_le_bytes());
    put(&mut v, 80, &(FAT_SECT as u32).to_le_bytes());
    put(&mut v, 84, &1u32.to_le_bytes());
    put(&mut v, 88, &(HEAP_SECT as u32).to_le_bytes());
    put(&mut v, 92, &16u32.to_le_bytes());
    put(&mut v, 96, &2u32.to_le_bytes());
    v[108] = 9;
    v[110] = 1;
    put(&mut v, 510, &[0x55, 0xAA]);
    set_fat(&mut v, 2, 0xFFFF_FFFF);
    set_fat(&mut v, 3, 0xFFFF_FFFF);
    set_fat(&mut v, 4, 0xFFFF_FFFF);
    set_fat(&mut v, 5, 6);
    set_fat(&mut v, 6, 0xFFFF_FFFF);

    let mut root = Vec::new();
    root.extend(entry_set("Relazione.txt", 0x20, 0x01, 3, 5, false));
    root.extend(entry_set("DOCS", 0x10, 0x01, 4, 512, false));
    root.extend(entry_set("vecchio.txt", 0x20, 0x01, 9, 1, true));
    root.extend([0x81u8; 32]); // voce bitmap: da ignorare
    put(&mut v, cluster_off(2), &root);
    put(&mut v, cluster_off(3), b"ciao!");

    let docs = entry_set("A.TXT", 0x20, 0x03, 5, 600, false);
    put(&mut v, cluster_off(4), &docs);
    put(&mut v, cluster_off(5), &[b'a'; 512]);
    put(&mut v, cluster_off(6), &[b'b'; 88]);
    v
}

fn find<'a>(list: &'a [DirEntry], name: &str) -> &'a DirEntry {
    list.iter().find(|e| e.name == name).expect("voce assente")
}

#[test]
fn apre_volume_exfat() {
    let v = volume();
    assert!(ExFat::open(&v).is_ok());
}

#[test]
fn lista_radice_ignora_cancellati_e_bitmap() {
    let v = volume();
    let fs = ExFat::open(&v).unwrap();
    let root = fs.root_dir().unwrap();
    assert_eq!(root.len(), 2);
    assert!(find(&root, "DOCS").is_dir);
    assert!(!root.iter().any(|e| e.name == "vecchio.txt"));
}

#[test]
fn legge_file_a_catena() {
    let v = volume();
    let fs = ExFat::open(&v).unwrap();
    let root = fs.root_dir().unwrap();
    assert_eq!(fs.read_file(find(&root, "Relazione.txt")).unwrap(), b"ciao!");
}

#[test]
fn legge_file_contiguo_su_piu_cluster() {
    let v = volume();
    let fs = ExFat::open(&v).unwrap();
    let root = fs.root_dir().unwrap();
    let docs = find(&root, "DOCS");
    let list = fs.read_dir(docs.first_cluster, docs.size, docs.contiguous).unwrap();
    let a = find(&list, "A.TXT").clone();
    assert!(a.contiguous);
    let data = fs.read_file(&a).unwrap();
    assert_eq!(data.len(), 600);
    assert!(data[..512].iter().all(|&c| c == b'a'));
    assert!(data[512..].iter().all(|&c| c == b'b'));
}

#[test]
fn checksum_sbagliato_salta_la_voce() {
    let mut v = volume();
    v[cluster_off(2) + 2] ^= 0xFF; // checksum della voce file di "Relazione.txt"
    let fs = ExFat::open(&v).unwrap();
    let root = fs.root_dir().unwrap();
    assert_eq!(root.len(), 1);
    assert!(!root.iter().any(|e| e.name == "Relazione.txt"));
}

#[test]
fn catena_con_ciclo_rilevata() {
    let mut v = volume();
    set_fat(&mut v, 3, 3); // 3 -> 3
    let fs = ExFat::open(&v).unwrap();
    let root = fs.root_dir().unwrap();
    let f = find(&root, "Relazione.txt").clone();
    assert_eq!(fs.read_file(&f), Err(ExFatError::Loop));
}

#[test]
fn cartella_non_si_legge_come_file() {
    let v = volume();
    let fs = ExFat::open(&v).unwrap();
    let root = fs.root_dir().unwrap();
    assert_eq!(fs.read_file(find(&root, "DOCS")), Err(ExFatError::IsDir));
}

#[test]
fn non_exfat_rifiutato() {
    let mut v = volume();
    v[3] = b'X';
    assert_eq!(ExFat::open(&v).err(), Some(ExFatError::NotExFat));
}

#[test]
fn firma_mancante_rifiutata() {
    let mut v = volume();
    v[510] = 0;
    assert_eq!(ExFat::open(&v).err(), Some(ExFatError::NoSignature));
}
