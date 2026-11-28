//! Test del lettore FAT32 in sola lettura. Il volume di prova è costruito qui dal codice.
use haiku_alloc_sim::fat32::*;

const BPS: usize = 512;
const RSVD: usize = 32;
const FATSZ: usize = 513;
const CLUSTERS: usize = 65600;
const TOT: usize = RSVD + 2 * FATSZ + CLUSTERS;
const EOC: u32 = 0x0FFF_FFFF;

fn put(v: &mut [u8], o: usize, b: &[u8]) {
    v[o..o + b.len()].copy_from_slice(b);
}
fn set_fat(v: &mut [u8], c: usize, val: u32) {
    put(v, RSVD * BPS + c * 4, &val.to_le_bytes());
}
fn cluster_off(c: usize) -> usize {
    (RSVD + 2 * FATSZ + (c - 2)) * BPS
}
/// Voce 8.3 (11 byte di nome, attributi, cluster, dimensione).
fn short(name: &[u8; 11], attr: u8, cluster: u32, size: u32) -> [u8; 32] {
    let mut e = [0u8; 32];
    e[0..11].copy_from_slice(name);
    e[11] = attr;
    e[20..22].copy_from_slice(&((cluster >> 16) as u16).to_le_bytes());
    e[26..28].copy_from_slice(&((cluster & 0xFFFF) as u16).to_le_bytes());
    e[28..32].copy_from_slice(&size.to_le_bytes());
    e
}
/// Voci LFN per `name` (ASCII), con il checksum del nome corto `sum`. Ordine su disco: ultima prima.
fn lfn(name: &str, sum: u8) -> Vec<[u8; 32]> {
    let units: Vec<u16> = name.encode_utf16().collect();
    let n = units.len().div_ceil(13);
    let offs = [1, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 28, 30];
    let mut out = Vec::new();
    for seq in (1..=n).rev() {
        let mut e = [0u8; 32];
        e[0] = seq as u8 | if seq == n { 0x40 } else { 0 };
        e[11] = 0x0F;
        e[13] = sum;
        for (i, &o) in offs.iter().enumerate() {
            let idx = (seq - 1) * 13 + i;
            let u = if idx < units.len() { units[idx] } else if idx == units.len() { 0 } else { 0xFFFF };
            put(&mut e, o, &u.to_le_bytes());
        }
        out.push(e);
    }
    out
}
fn write_entries(v: &mut [u8], cluster: usize, entries: &[[u8; 32]]) {
    for (i, e) in entries.iter().enumerate() {
        put(v, cluster_off(cluster) + i * 32, e);
    }
}

/// Volume di prova:
/// root (cluster 2): "Relazione-lunga.txt" (LFN, cluster 3, "ciao!"), "DOCS" (cluster 4).
/// DOCS: ".", "..", "A.TXT" (cluster 5->6, 600 byte: 512 'a' + 88 'b').
fn volume() -> Vec<u8> {
    let mut v = vec![0u8; TOT * BPS];
    put(&mut v, 11, &(BPS as u16).to_le_bytes());
    v[13] = 1;
    put(&mut v, 14, &(RSVD as u16).to_le_bytes());
    v[16] = 2;
    put(&mut v, 32, &(TOT as u32).to_le_bytes());
    put(&mut v, 36, &(FATSZ as u32).to_le_bytes());
    put(&mut v, 44, &2u32.to_le_bytes());
    put(&mut v, 510, &[0x55, 0xAA]);
    set_fat(&mut v, 2, EOC);
    set_fat(&mut v, 3, EOC);
    set_fat(&mut v, 4, EOC);
    set_fat(&mut v, 5, 6);
    set_fat(&mut v, 6, EOC);

    let short_sum = short_checksum(b"RELAZ~1 TXT");
    let mut root = lfn("Relazione-lunga.txt", short_sum);
    root.push(short(b"RELAZ~1 TXT", 0x20, 3, 5));
    root.push(short(b"DOCS       ", 0x10, 4, 0));
    write_entries(&mut v, 2, &root);
    put(&mut v, cluster_off(3), b"ciao!");

    let docs = [
        short(b".          ", 0x10, 4, 0),
        short(b"..         ", 0x10, 2, 0),
        short(b"A       TXT", 0x20, 5, 600),
    ];
    write_entries(&mut v, 4, &docs);
    put(&mut v, cluster_off(5), &[b'a'; 512]);
    put(&mut v, cluster_off(6), &[b'b'; 88]);
    v
}

fn find<'a>(list: &'a [DirEntry], name: &str) -> &'a DirEntry {
    list.iter().find(|e| e.name == name).expect("voce assente")
}

#[test]
fn apre_volume_fat32() {
    let v = volume();
    assert!(Fat32::open(&v).is_ok());
}

#[test]
fn legge_nome_lungo_lfn_e_contenuto() {
    let v = volume();
    let fs = Fat32::open(&v).unwrap();
    let root = fs.read_dir(2).unwrap();
    let f = find(&root, "Relazione-lunga.txt");
    assert_eq!(fs.read_file(f).unwrap(), b"ciao!");
}

#[test]
fn checksum_lfn_sbagliato_ricade_sul_nome_corto() {
    let mut v = volume();
    let off = cluster_off(2); // prima voce LFN: il suo checksum è al byte 13
    v[off + 13] ^= 0xFF;
    let fs = Fat32::open(&v).unwrap();
    let root = fs.read_dir(2).unwrap();
    assert!(root.iter().any(|e| e.name == "RELAZ~1.TXT"));
}

#[test]
fn cartella_e_file_su_piu_cluster() {
    let v = volume();
    let fs = Fat32::open(&v).unwrap();
    let root = fs.read_dir(2).unwrap();
    let docs = find(&root, "DOCS");
    assert!(docs.is_dir);
    let list = fs.read_dir(docs.first_cluster).unwrap();
    assert_eq!(list.len(), 1, "'.' e '..' non devono comparire");
    let a = fs.read_file(&list[0]).unwrap();
    assert_eq!(a.len(), 600);
    assert!(a[..512].iter().all(|&c| c == b'a'));
    assert!(a[512..].iter().all(|&c| c == b'b'));
}

#[test]
fn cartella_non_si_legge_come_file() {
    let v = volume();
    let fs = Fat32::open(&v).unwrap();
    let root = fs.read_dir(2).unwrap();
    assert_eq!(fs.read_file(find(&root, "DOCS")), Err(FatError::IsDir));
}

#[test]
fn catena_con_ciclo_rilevata() {
    let mut v = volume();
    set_fat(&mut v, 6, 5); // 5 -> 6 -> 5
    let fs = Fat32::open(&v).unwrap();
    assert_eq!(fs.chain(5), Err(FatError::Loop));
}

#[test]
fn volume_piccolo_non_e_fat32() {
    let mut v = volume();
    put(&mut v, 32, &5000u32.to_le_bytes());
    assert_eq!(Fat32::open(&v).err(), Some(FatError::NotFat32));
}

#[test]
fn firma_mancante_rifiutata() {
    let mut v = volume();
    v[510] = 0;
    assert_eq!(Fat32::open(&v).err(), Some(FatError::NoSignature));
}
