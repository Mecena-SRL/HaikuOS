//! Round-trip, limiti di slot e rilevamento di manomissioni per LZSS + Ascon-AEAD128.

use haiku_alloc_sim::lzss::{self, LzssError};
use haiku_alloc_sim::slot::{self, SlotError};
use haiku_alloc_sim::SLOT_BYTES;

const KEY: [u8; 16] = *b"chiave-di-prova!";
const NONCE: [u8; 16] = [7; 16];

/// Generatore pseudo-casuale deterministico (xorshift), senza dipendenze.
fn noise(n: usize, seed: u64) -> Vec<u8> {
    let mut x = seed | 1;
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect()
}

#[test]
fn lzss_round_trip() {
    let samples: Vec<Vec<u8>> = vec![
        vec![],
        b"a".to_vec(),
        b"Haiku OS: memoria compressa e cifrata.".to_vec(),
        vec![0u8; SLOT_BYTES],
        noise(2000, 42),
    ];
    for s in samples {
        let c = lzss::compress(&s);
        assert_eq!(lzss::decompress(&c, SLOT_BYTES).unwrap(), s);
    }
}

#[test]
fn lzss_repetitive_data_shrinks_a_lot() {
    let zeros = vec![0u8; SLOT_BYTES];
    // Massimo 10 byte per 12 bit: circa 15% per dati ripetitivi.
    assert!(lzss::compress(&zeros).len() < SLOT_BYTES / 5);
}

#[test]
fn lzss_refuses_declared_length_above_limit() {
    let c = lzss::compress(&vec![1u8; 100]);
    assert_eq!(lzss::decompress(&c, 99), Err(LzssError::TooLarge));
}

#[test]
fn lzss_rejects_truncated_stream() {
    let c = lzss::compress(&noise(500, 9));
    assert_eq!(lzss::decompress(&c[..c.len() / 2], SLOT_BYTES), Err(LzssError::Corrupt));
}

#[test]
fn slot_round_trip() {
    let plain = b"stato del programma: livello 3, punti 1200".repeat(20);
    let sealed = slot::seal(&plain, &KEY, &NONCE, 1, SLOT_BYTES).unwrap();
    assert_eq!(slot::open(&sealed, &KEY, 1, SLOT_BYTES).unwrap(), plain);
}

#[test]
fn slot_rejects_data_that_does_not_fit() {
    // Il rumore si espande con LZSS (9 bit per byte): 32 KB casuali non entrano nello slot.
    let plain = noise(SLOT_BYTES, 3);
    assert_eq!(slot::seal(&plain, &KEY, &NONCE, 1, SLOT_BYTES), Err(SlotError::TooLarge));
}

#[test]
fn slot_detects_tampering() {
    let mut sealed = slot::seal(b"dati importanti", &KEY, &NONCE, 2, SLOT_BYTES).unwrap();
    let last = sealed.len() - 1;
    sealed[last] ^= 0x01;
    assert_eq!(slot::open(&sealed, &KEY, 2, SLOT_BYTES), Err(SlotError::Auth));
}

#[test]
fn slot_binds_to_slot_number() {
    let sealed = slot::seal(b"dati", &KEY, &NONCE, 2, SLOT_BYTES).unwrap();
    assert_eq!(slot::open(&sealed, &KEY, 3, SLOT_BYTES), Err(SlotError::Auth));
}

#[test]
fn slot_rejects_wrong_key() {
    let sealed = slot::seal(b"dati", &KEY, &NONCE, 2, SLOT_BYTES).unwrap();
    let other = *b"altra-chiave-123";
    assert_eq!(slot::open(&sealed, &other, 2, SLOT_BYTES), Err(SlotError::Auth));
}
