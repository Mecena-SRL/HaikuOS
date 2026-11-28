//! Test host: `cd gba && rustc --edition 2021 --test host/test_scale.rs -o /tmp/scale_t && /tmp/scale_t`
#![allow(dead_code)]
#[path = "../src/scale.rs"]
mod scale;

use scale::{factor, upscale, LH, LW};

const BG: u16 = 0;
const A: u16 = 7;

#[test]
fn fattore_intero() {
    assert_eq!(factor(240, 160), 1);
    assert_eq!(factor(1280, 800), 5);
    assert_eq!(factor(1920, 1080), 6);
    assert_eq!(factor(100, 100), 1);
}

#[test]
fn stessa_misura_copia_identica() {
    let src = vec![A; LW * LH];
    let mut dst = vec![BG; LW * LH];
    assert!(upscale(&src, &mut dst, LW, LH, BG));
    assert_eq!(dst, src);
}

#[test]
fn ingrandimento_intero_centrato() {
    let (w, h) = (1280, 800); // k = 5, area 1200×800, bande 40 px a sinistra e a destra
    let mut src = vec![A; LW * LH];
    src[0] = 3; // angolo in alto a sinistra
    src[LW * LH - 1] = 9; // angolo in basso a destra
    let mut dst = vec![BG; w * h];
    assert!(upscale(&src, &mut dst, w, h, BG));
    assert_eq!(dst[0], BG); // banda sinistra
    assert_eq!(dst[40], 3); // primo blocco 5×5
    assert_eq!(dst[4 * w + 44], 3); // ultimo pixel del blocco
    assert_eq!(dst[5 * w + 40], A); // blocco successivo, riga sotto
    assert_eq!(dst[799 * w + 1239], 9); // ultimo pixel
    assert_eq!(dst[799 * w + 1240], BG); // banda destra
    let n = dst.iter().filter(|&&p| p != BG).count();
    assert_eq!(n, LW * LH * 25); // ogni pixel sorgente diventa 5×5, nessuno è BG
}

#[test]
fn misure_sbagliate_rifiutate() {
    let src = vec![A; LW * LH];
    let mut small = vec![BG; 100 * 100];
    assert!(!upscale(&src, &mut small, 100, 100, BG));
    let mut dst = vec![BG; 1280 * 800];
    assert!(!upscale(&src[1..], &mut dst, 1280, 800, BG));
}
