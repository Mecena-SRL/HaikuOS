use haiku_alloc_sim::opentype::*;

/// Offset dei dati di una tabella (per modificare i byte nei test).
fn off_of(d: &[u8], tag: &[u8; 4]) -> usize {
    let r = (0..3).map(|i| 12 + 16 * i).find(|&r| &d[r..r + 4] == tag).unwrap();
    u32::from_be_bytes(d[r + 8..r + 12].try_into().unwrap()) as usize
}

#[test]
fn legge_famiglia_upem_e_glifi() {
    let f = parse(&synth_font("Prova Sans")).unwrap();
    assert_eq!(f, FontInfo { family: "Prova Sans".into(), units_per_em: 1000, num_glyphs: 42 });
}

#[test]
fn versione_errata() {
    let mut d = synth_font("X");
    d[0..4].copy_from_slice(&0x1234_5678u32.to_be_bytes());
    assert_eq!(parse(&d), Err(OtError::BadVersion));
}

#[test]
fn magic_errato() {
    let mut d = synth_font("X");
    let h = off_of(&d, b"head");
    d[h + 12] ^= 0xFF;
    assert_eq!(parse(&d), Err(OtError::BadMagic));
}

#[test]
fn tabella_mancante_e_file_troncato() {
    let d = synth_font("X");
    let mut no_maxp = d.clone();
    no_maxp[12 + 16..12 + 16 + 4].copy_from_slice(b"zzzz");
    assert_eq!(parse(&no_maxp), Err(OtError::MissingTable(*b"maxp")));
    assert_eq!(parse(&d[..d.len() - 3]), Err(OtError::Truncated));
    assert_eq!(parse(&d[..8]), Err(OtError::MissingTable(*b"head")));
    assert_eq!(parse(&[]), Err(OtError::Truncated));
}

#[test]
fn offset_tabella_oltre_il_file() {
    let mut d = synth_font("X");
    d[12 + 8..12 + 12].copy_from_slice(&0xFFFF_FFF0u32.to_be_bytes());
    assert_eq!(parse(&d), Err(OtError::Truncated));
}

#[test]
fn ripiego_su_nome_macintosh() {
    let mut d = synth_font("Abc");
    let n = off_of(&d, b"name");
    d[n + 6..n + 8].copy_from_slice(&1u16.to_be_bytes()); // piattaforma 1 (Macintosh)
    d[n + 14..n + 16].copy_from_slice(&3u16.to_be_bytes()); // lunghezza 3
    d[n + 18..n + 21].copy_from_slice(b"Abc");
    assert_eq!(parse(&d).unwrap().family, "Abc");
}

#[test]
fn nome_assente() {
    let mut d = synth_font("Abc");
    let n = off_of(&d, b"name");
    d[n + 12..n + 14].copy_from_slice(&2u16.to_be_bytes()); // nameID 2: non è la famiglia
    assert_eq!(parse(&d), Err(OtError::NoFamilyName));
}
