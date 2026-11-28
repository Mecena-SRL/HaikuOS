// Font vettoriale: metriche, copertura e antialiasing.
mod common;
use common::Buf;
use haiku_gui::vfont::{advance, draw_text, raster};
use haiku_gui::Tok;

#[test]
fn passo_dei_caratteri_e_esatto() {
    assert_eq!(advance(64, "AB"), 96);
    assert_eq!(advance(16, "ABC"), 36);
    assert_eq!(advance(8, "A"), 6);
    assert_eq!(advance(13, ""), 0);
}

#[test]
fn draw_text_ritorna_la_larghezza_di_advance() {
    let mut b = Buf::new(200, 40);
    let w = draw_text(&mut b, 0, 0, 16, "HAIKU", Tok::T1);
    assert_eq!(w, advance(16, "HAIKU"));
}

#[test]
fn spazio_non_disegna_nulla() {
    let mut b = Buf::new(40, 40);
    draw_text(&mut b, 0, 0, 32, "   ", Tok::T1);
    assert!(b.alpha.iter().all(|&a| a == 0));
}

#[test]
fn rettangolo_a_piena_copertura() {
    let mut b = Buf::new(40, 40);
    // Unità = px a size 64: rettangolo 16×8 in (10,10).
    raster(&mut b, &[4, 0, 0, 16, 8], 10, 10, 64, Tok::T1);
    assert_eq!(b.alpha[(10 * 40 + 10) as usize], 255);
    assert_eq!(b.alpha[(17 * 40 + 25) as usize], 255);
    assert_eq!(b.alpha[(10 * 40 + 9) as usize], 0);
    assert_eq!(b.alpha[(10 * 40 + 26) as usize], 0);
    assert_eq!(b.alpha[(18 * 40 + 10) as usize], 0);
}

#[test]
fn bordo_a_mezzo_pixel_da_copertura_128() {
    let mut b = Buf::new(40, 40);
    // size 32: 1 unità = 0.5 px. Il bordo sinistro cade a metà del pixel 10.
    raster(&mut b, &[4, 1, 0, 16, 8], 10, 10, 32, Tok::T1);
    let at = |x: i32, y: i32| b.alpha[(y * 40 + x) as usize];
    assert_eq!(at(10, 11), 128);
    assert_eq!(at(11, 11), 255);
    assert_eq!(at(18, 11), 0);
}

#[test]
fn area_coperta_e_esatta() {
    let mut b = Buf::new(40, 40);
    raster(&mut b, &[4, 0, 0, 16, 8], 0, 0, 64, Tok::T1);
    let somma: u32 = b.alpha.iter().map(|&a| a as u32).sum();
    assert_eq!(somma, 16 * 8 * 255);
}

#[test]
fn curva_quadratica_riempie_la_parte_giusta() {
    // Segmento di parabola: M(0,64) Q(32,0)→(64,64), chiuso sulla base.
    let ops = [0, 0, 64, 2, 32, 0, 64, 64, 3];
    let mut b = Buf::new(64, 64);
    raster(&mut b, &ops, 0, 0, 64, Tok::T1);
    let at = |x: i32, y: i32| b.alpha[(y * 64 + x) as usize];
    assert_eq!(at(32, 50), 255);
    assert_eq!(at(5, 58), 255);
    assert_eq!(at(32, 20), 0);
    assert_eq!(at(5, 40), 0);
}

#[test]
fn testo_produce_bordi_antialiasati() {
    let mut b = Buf::new(40, 40);
    draw_text(&mut b, 2, 2, 13, "A", Tok::T1);
    let parziali = b.alpha.iter().filter(|&&a| a > 0 && a < 255).count();
    let piene = b.alpha.iter().filter(|&&a| a == 255).count();
    assert!(parziali > 0, "nessun pixel parziale");
    assert!(piene > 0, "nessun pixel pieno");
}

#[test]
fn ops_malformati_non_bloccano() {
    let mut b = Buf::new(16, 16);
    raster(&mut b, &[0, 1], 0, 0, 16, Tok::T1);
    raster(&mut b, &[4, 0, 0], 0, 0, 16, Tok::T1);
    raster(&mut b, &[9, 9, 9], 0, 0, 16, Tok::T1);
    assert!(b.alpha.iter().all(|&a| a == 0));
}

#[test]
fn font_di_default_esistono() {
    use haiku_gui::vfont::{default_font, Role};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for r in [Role::Ui, Role::Mono, Role::Reading] {
        let p = root.join(default_font(r));
        assert!(p.is_file(), "manca {}", p.display());
    }
}
