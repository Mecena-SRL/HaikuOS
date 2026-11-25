//! Test della lettura MBR e GPT. Le immagini di prova sono costruite qui dal codice, nessun file esterno.
use haiku_alloc_sim::partition::*;

const SECTORS: usize = 2048;

fn put(d: &mut [u8], o: usize, b: &[u8]) {
    d[o..o + b.len()].copy_from_slice(b);
}

/// GUID testuale -> 16 byte su disco (inverso di guid_to_string).
fn guid(s: &str) -> [u8; 16] {
    let p: Vec<&str> = s.split('-').collect();
    let mut o = [0u8; 16];
    o[0..4].copy_from_slice(&u32::from_str_radix(p[0], 16).unwrap().to_le_bytes());
    o[4..6].copy_from_slice(&u16::from_str_radix(p[1], 16).unwrap().to_le_bytes());
    o[6..8].copy_from_slice(&u16::from_str_radix(p[2], 16).unwrap().to_le_bytes());
    let rest = format!("{}{}", p[3], p[4]);
    for i in 0..8 {
        o[8 + i] = u8::from_str_radix(&rest[2 * i..2 * i + 2], 16).unwrap();
    }
    o
}

fn mbr_entry(d: &mut [u8], i: usize, kind: u8, lba: u32, n: u32) {
    let o = 446 + i * 16;
    d[o + 4] = kind;
    put(d, o + 8, &lba.to_le_bytes());
    put(d, o + 12, &n.to_le_bytes());
}

fn mbr_sig(d: &mut [u8]) {
    put(d, 510, &[0x55, 0xAA]);
}

/// Disco GPT con MBR protettiva. `parts`: (tipo, first, last, nome).
fn gpt_disk(parts: &[(&str, u64, u64, &str)]) -> Vec<u8> {
    let mut d = vec![0u8; SECTORS * SECTOR];
    mbr_entry(&mut d, 0, MBR_KIND_GPT_PROTECTIVE, 1, (SECTORS - 1) as u32);
    mbr_sig(&mut d);
    let mut arr = vec![0u8; GPT_MAX_ENTRIES * GPT_ENTRY_BYTES];
    for (i, (t, f, l, name)) in parts.iter().enumerate() {
        let e = &mut arr[i * 128..(i + 1) * 128];
        e[0..16].copy_from_slice(&guid(t));
        e[16..32].copy_from_slice(&guid("11111111-2222-3333-4444-555566667777"));
        e[32..40].copy_from_slice(&f.to_le_bytes());
        e[40..48].copy_from_slice(&l.to_le_bytes());
        for (j, u) in name.encode_utf16().enumerate() {
            e[56 + 2 * j..58 + 2 * j].copy_from_slice(&u.to_le_bytes());
        }
    }
    put(&mut d, 2 * SECTOR, &arr);
    let mut h = vec![0u8; SECTOR];
    put(&mut h, 0, b"EFI PART");
    put(&mut h, 8, &0x0001_0000u32.to_le_bytes());
    put(&mut h, 12, &92u32.to_le_bytes());
    put(&mut h, 24, &1u64.to_le_bytes());
    put(&mut h, 32, &((SECTORS - 1) as u64).to_le_bytes());
    put(&mut h, 40, &34u64.to_le_bytes());
    put(&mut h, 48, &((SECTORS - 34) as u64).to_le_bytes());
    put(&mut h, 72, &2u64.to_le_bytes());
    put(&mut h, 80, &(GPT_MAX_ENTRIES as u32).to_le_bytes());
    put(&mut h, 84, &(GPT_ENTRY_BYTES as u32).to_le_bytes());
    put(&mut h, 88, &crc32(&arr).to_le_bytes());
    let c = crc32(&h[..92]);
    put(&mut h, 16, &c.to_le_bytes());
    put(&mut d, SECTOR, &h);
    d
}

const EFI: &str = "C12A7328-F81F-11D2-BA4B-00A0C93EC93B";
const BASIC: &str = "EBD0A0A2-B9E5-4433-87C0-68B6B72699C7";

#[test]
fn crc32_valore_noto() {
    // Vettore di test standard CRC32 IEEE.
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
}

#[test]
fn guid_round_trip() {
    assert_eq!(guid_to_string(&guid(EFI)), EFI);
}

#[test]
fn mbr_legge_voci_non_vuote() {
    let mut d = vec![0u8; SECTOR];
    mbr_entry(&mut d, 0, 0x0C, 2048, 4096);
    mbr_entry(&mut d, 2, 0x83, 8192, 100);
    mbr_sig(&mut d);
    let v = read_mbr(&d).unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v[0], MbrEntry { kind: 0x0C, first_lba: 2048, sectors: 4096 });
    assert!(!is_gpt_protective(&v));
}

#[test]
fn mbr_senza_firma_rifiutata() {
    assert_eq!(read_mbr(&vec![0u8; SECTOR]), Err(PartError::NoSignature));
    assert_eq!(read_mbr(&[0u8; 100]), Err(PartError::TooSmall));
}

#[test]
fn gpt_legge_partizioni() {
    let d = gpt_disk(&[(EFI, 34, 233, "EFI"), (BASIC, 234, 1000, "Dati")]);
    let p = read_gpt(&d).unwrap();
    assert_eq!(p.len(), 2);
    assert_eq!(p[0].type_guid, EFI);
    assert_eq!((p[0].first_lba, p[0].last_lba), (34, 233));
    assert_eq!(p[1].name, "Dati");
    assert_eq!(p[1].unique_guid, "11111111-2222-3333-4444-555566667777");
}

#[test]
fn gpt_protective_mbr_riconosciuta() {
    let d = gpt_disk(&[(BASIC, 34, 100, "x")]);
    assert!(is_gpt_protective(&read_mbr(&d).unwrap()));
}

#[test]
fn gpt_intestazione_alterata_rilevata() {
    let mut d = gpt_disk(&[(BASIC, 34, 100, "x")]);
    d[SECTOR + 56] ^= 0xFF; // GUID del disco: CRC dell'intestazione non torna
    assert_eq!(read_gpt(&d), Err(PartError::BadCrc));
}

#[test]
fn gpt_array_voci_alterato_rilevato() {
    let mut d = gpt_disk(&[(BASIC, 34, 100, "x")]);
    d[2 * SECTOR + 40] ^= 0x01; // ultimo LBA della voce 0
    assert_eq!(read_gpt(&d), Err(PartError::BadCrc));
}

#[test]
fn gpt_partizione_fuori_area_rifiutata() {
    // Ultimo LBA oltre l'area utilizzabile: va rifiutato anche con CRC validi.
    let d = gpt_disk(&[(BASIC, 34, (SECTORS - 10) as u64, "x")]);
    assert_eq!(read_gpt(&d), Err(PartError::OutOfRange));
}

#[test]
fn gpt_firma_assente_rifiutata() {
    let mut d = gpt_disk(&[]);
    d[SECTOR] = b'X';
    assert_eq!(read_gpt(&d), Err(PartError::NoSignature));
}

#[test]
fn gpt_disco_vuoto_senza_partizioni() {
    assert_eq!(read_gpt(&gpt_disk(&[])).unwrap(), Vec::new());
}
