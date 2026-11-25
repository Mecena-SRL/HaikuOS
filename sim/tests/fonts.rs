use haiku_alloc_sim::fonts::*;

fn with_fonts(n: usize) -> (FontManager, Vec<FontId>) {
    let mut m = FontManager::new();
    let ids = (0..n)
        .map(|i| m.install(&format!("F{i}"), Format::Bitmap).unwrap())
        .collect();
    (m, ids)
}

#[test]
fn installa_non_attiva_e_rifiuta_duplicati() {
    let (mut m, ids) = with_fonts(1);
    assert!(!m.is_active(ids[0]));
    assert_eq!(m.install("F0", Format::Vector), Err(FontError::Duplicate));
    assert_eq!(m.format(ids[0]), Ok(Format::Bitmap));
}

#[test]
fn registro_pieno_e_nome_lungo() {
    let (mut m, _) = with_fonts(MAX_FONTS);
    assert_eq!(m.install("nuovo", Format::Bitmap), Err(FontError::Full));
    let mut m = FontManager::new();
    let lungo = "x".repeat(MAX_NAME + 1);
    assert_eq!(
        m.install(&lungo, Format::Bitmap),
        Err(FontError::NameTooLong)
    );
}

#[test]
fn attiva_disattiva_con_limite() {
    let (mut m, ids) = with_fonts(MAX_ACTIVE + 1);
    for &id in &ids[..MAX_ACTIVE] {
        m.activate(id).unwrap();
    }
    assert_eq!(m.activate(ids[MAX_ACTIVE]), Err(FontError::TooManyActive));
    m.activate(ids[0]).unwrap(); // già attivo: nessun errore
    m.deactivate(ids[0]).unwrap();
    assert!(m.activate(ids[MAX_ACTIVE]).is_ok());
}

#[test]
fn id_sconosciuto_negato() {
    let (mut m, _) = with_fonts(1);
    assert_eq!(m.activate(FontId(9)), Err(FontError::Unknown));
    assert_eq!(m.preview(FontId(9), "ciao"), Err(FontError::Unknown));
    assert!(!m.is_active(FontId(9)));
}

#[test]
fn font_di_sistema_attiva_e_non_disattivabile() {
    let (mut m, ids) = with_fonts(2);
    m.set_system(ids[1]).unwrap();
    assert_eq!(m.system(), Some(ids[1]));
    assert!(m.is_active(ids[1]));
    assert_eq!(m.deactivate(ids[1]), Err(FontError::SystemFont));
    m.deactivate(ids[0]).unwrap(); // gli altri sì
}

#[test]
fn anteprima_solo_su_font_installato() {
    let (m, ids) = with_fonts(1);
    assert_eq!(m.preview(ids[0], "ABC 123").as_deref(), Ok("ABC 123"));
    let lungo = "a".repeat(MAX_SAMPLE + 1);
    assert_eq!(m.preview(ids[0], &lungo), Err(FontError::SampleTooLong));
}

#[test]
fn collezioni_membri_e_limiti() {
    let (mut m, ids) = with_fonts(2);
    let c = m.create_collection("Giochi").unwrap();
    m.add_to_collection(c, ids[0]).unwrap();
    m.add_to_collection(c, ids[0]).unwrap(); // doppio: nessun errore, nessun duplicato
    assert_eq!(m.collection(c).unwrap().1, &[ids[0]]);
    // Un font può stare in più collezioni.
    let d = m.create_collection("Doc").unwrap();
    m.add_to_collection(d, ids[0]).unwrap();
    assert_eq!(m.collection(d).unwrap().1, &[ids[0]]);
    // Id sconosciuto e collezione inesistente negati.
    assert_eq!(m.add_to_collection(c, FontId(9)), Err(FontError::Unknown));
    assert_eq!(m.add_to_collection(99, ids[0]), Err(FontError::Unknown));
}

#[test]
fn collezioni_piene() {
    let (mut m, ids) = with_fonts(MAX_MEMBERS + 1);
    let c = m.create_collection("Tutti").unwrap();
    for &id in &ids[..MAX_MEMBERS] {
        m.add_to_collection(c, id).unwrap();
    }
    assert_eq!(
        m.add_to_collection(c, ids[MAX_MEMBERS]),
        Err(FontError::MemberFull)
    );
    for i in 1..MAX_COLLECTIONS {
        m.create_collection(&format!("C{i}")).unwrap();
    }
    assert_eq!(m.create_collection("oltre"), Err(FontError::CollectionFull));
}
