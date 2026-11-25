//! Test host: `cd gba && rustc --edition 2021 --test host/test_ascon.rs -o /tmp/ascon_t && /tmp/ascon_t`
//! Vettori generati con `ascon-aead` 0.6 (`AsconAead128`, lo stesso usato da sim/src/slot.rs).
#![allow(dead_code)]
#[path = "../src/ascon.rs"]
mod ascon;

use ascon::{open, open_block, seal, seal_block};

const KEY: [u8; 16] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
];
const NONCE: [u8; 16] = [
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
}

fn plain(n: usize) -> Vec<u8> {
    (0..n).map(|i| i as u8).collect()
}

fn kat(ad: &[u8], n: usize, expect_hex: &str) {
    let pt = plain(n);
    let mut out = vec![0u8; n + 16];
    assert_eq!(seal(&KEY, &NONCE, ad, &pt, &mut out), n + 16);
    assert_eq!(out, unhex(expect_hex), "KAT n={} ad_len={}", n, ad.len());
}

#[test]
fn kat_pt_vuoto() {
    kat(b"", 0, "4f9c278211bec9316bf68f46ee8b2ec6");
}

#[test]
fn kat_pt_5() {
    kat(b"", 5, "c8e3fece0466412dd4105ef170e27fe83f0c77b97f");
}

#[test]
fn kat_pt_64() {
    kat(
        b"",
        64,
        "c8e3fece044ce5cac3c8521118b7829b1520ff79648f51124105cb5eb58eb4befaaf48c955a72bf4b8b90f640f50ef4333d3e5a5f3dedd13965b863289b704209a507043dc8e09b0293607d4475bce42",
    );
}

#[test]
fn kat_pt_100() {
    kat(
        b"",
        100,
        "c8e3fece044ce5cac3c8521118b7829b1520ff79648f51124105cb5eb58eb4befaaf48c955a72bf4b8b90f640f50ef4333d3e5a5f3dedd13965b863289b704201efdcf6dc638aad8c2053521137f9ae9f8d36b54000a7c4c897c2a1a96734817c1cc9129136250f0ba7f320cc9e56e7410121e7d",
    );
}

#[test]
fn kat_con_ad() {
    kat(b"ABCDEFGH", 5, "190bb4f9a3a8b2b9cf8da5f7952ae119fc94904e05");
}

#[test]
fn round_trip_lunghezze() {
    for ad in [&b""[..], &b"x"[..], &b"ABCDEFGH"[..], &b"0123456789abcdefXYZ"[..]] {
        for n in 0..=40 {
            let pt = plain(n);
            let mut ct = vec![0u8; n + 16];
            assert_eq!(seal(&KEY, &NONCE, ad, &pt, &mut ct), n + 16);
            let mut back = vec![0xAAu8; n];
            assert_eq!(open(&KEY, &NONCE, ad, &ct, &mut back), Some(n), "n={}", n);
            assert_eq!(back, pt);
        }
    }
}

#[test]
fn out_troppo_corto() {
    let mut out = [0u8; 10];
    assert_eq!(seal(&KEY, &NONCE, b"", &[1, 2, 3], &mut out), 0);
    let mut out2 = [0u8; 2];
    assert_eq!(open(&KEY, &NONCE, b"", &[0u8; 16], &mut out2), None);
}

#[test]
fn tag_manomesso_none_e_azzera() {
    let pt = plain(20);
    let mut ct = vec![0u8; 36];
    seal(&KEY, &NONCE, b"ad", &pt, &mut ct);
    // Bit del tag (ultimo byte).
    let mut bad = ct.clone();
    bad[35] ^= 0x01;
    let mut out = vec![0x55u8; 20];
    assert_eq!(open(&KEY, &NONCE, b"ad", &bad, &mut out), None);
    assert!(out.iter().all(|&b| b == 0), "plaintext non azzerato dopo errore");
    // Bit del ciphertext.
    let mut bad2 = ct.clone();
    bad2[3] ^= 0x80;
    assert_eq!(open(&KEY, &NONCE, b"ad", &bad2, &mut out), None);
    // AD diverso.
    assert_eq!(open(&KEY, &NONCE, b"AD", &ct, &mut out), None);
    // Chiave diversa.
    let mut k2 = KEY;
    k2[0] ^= 1;
    assert_eq!(open(&k2, &NONCE, b"ad", &ct, &mut out), None);
    // Originale ancora valido.
    assert_eq!(open(&KEY, &NONCE, b"ad", &ct, &mut out), Some(20));
    assert_eq!(out, pt);
}

#[test]
fn blocco_round_trip() {
    let mut pt = [0u8; 64];
    for (i, b) in pt.iter_mut().enumerate() {
        *b = (i * 7 + 3) as u8;
    }
    let mut ct = [0u8; 80];
    seal_block(&KEY, 1, 2, 3, &pt, &mut ct);
    let mut back = [0u8; 64];
    assert!(open_block(&KEY, 1, 2, 3, &ct, &mut back));
    assert_eq!(back, pt);
    // Il blocco e' legato alla sua posizione: indice o generazione diversi falliscono.
    assert!(!open_block(&KEY, 1, 2, 4, &ct, &mut back));
    assert!(!open_block(&KEY, 2, 2, 3, &ct, &mut back));
    assert!(!open_block(&KEY, 1, 9, 3, &ct, &mut back));
    // Tag manomesso.
    let mut bad = ct;
    bad[79] ^= 0x10;
    assert!(!open_block(&KEY, 1, 2, 3, &bad, &mut back));
}

#[test]
fn blocco_coerente_con_seal_generico() {
    // seal_block deve coincidere con seal() sul nonce gen||file||block||0000.
    let pt = [0x5Au8; 64];
    let mut a = [0u8; 80];
    seal_block(&KEY, 0x0102_0304, 0x0506_0708, 9, &pt, &mut a);
    let mut nonce = [0u8; 16];
    nonce[0..4].copy_from_slice(&0x0102_0304u32.to_le_bytes());
    nonce[4..8].copy_from_slice(&0x0506_0708u32.to_le_bytes());
    nonce[8..12].copy_from_slice(&9u32.to_le_bytes());
    let mut b = [0u8; 80];
    assert_eq!(seal(&KEY, &nonce, b"", &pt, &mut b), 80);
    assert_eq!(a, b);
}
