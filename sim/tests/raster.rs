use haiku_alloc_sim::opentype::{self, synth_font};
use haiku_alloc_sim::raster::render_text;

// Con 10 px/em il quadrato 'A' (x 100..600, y 0..700) occupa colonne 1..5, righe 3..9.

#[test]
fn ids_e_avanzi() {
    let d = synth_font("T");
    assert_eq!(opentype::glyph_id(&d, 'A').unwrap(), 1);
    assert_eq!(opentype::glyph_id(&d, 'Q').unwrap(), 0);
    assert_eq!(opentype::advance(&d, 1).unwrap(), 600);
    assert!(opentype::outline(&d, 0).unwrap().is_none());
}

#[test]
fn disegna_il_quadrato() {
    let c = render_text(&synth_font("T"), "A", 10.0).unwrap();
    assert_eq!((c.w, c.h), (6, 10));
    assert!(c.px[5 * c.w + 3]); // centro
    assert!(!c.px[5 * c.w]); // colonna 0: fuori
    assert!(!c.px[2 * c.w + 3]); // riga 2: sopra
    assert!(c.px[9 * c.w + 3]); // riga 9: ultima
    assert_eq!(c.px.iter().filter(|&&p| p).count(), 5 * 7);
}

#[test]
fn testo_e_glifo_mancante() {
    let d = synth_font("T");
    let c = render_text(&d, "AA", 10.0).unwrap();
    assert_eq!(c.w, 12);
    assert_eq!(c.px.iter().filter(|&&p| p).count(), 2 * 5 * 7);
    // 'Z' non ha glifo: avanza (500 unità) ma non disegna.
    let z = render_text(&d, "Z", 10.0).unwrap();
    assert_eq!(z.w, 5);
    assert!(z.px.iter().all(|&p| !p));
    assert_eq!(render_text(&d, "", 10.0).unwrap().w, 0);
}

#[test]
fn art_ha_una_riga_per_pixel() {
    let art = render_text(&synth_font("T"), "A", 10.0).unwrap().to_art();
    assert_eq!(art.len(), 10);
    assert_eq!(art[5], " #####");
    assert_eq!(art[0], "      ");
}
