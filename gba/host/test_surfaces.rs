//! Test host: `cd gba && rustc --edition 2021 --test host/test_surfaces.rs -o /tmp/surf_t && /tmp/surf_t`
#![allow(dead_code)]
#[path = "../src/compositor.rs"]
mod compositor;
#[path = "../src/surfaces.rs"]
mod surfaces;

use surfaces::{Err, Quota, Surfaces, SPIX, SURF_BYTES};

/// Quota finta: budget in byte, offset crescente, nessun controllo di sovrapposizione.
struct Budget {
    left: usize,
    next: usize,
}
impl Quota for Budget {
    fn take(&mut self, _p: usize, bytes: usize) -> Option<usize> {
        if bytes > self.left {
            return None;
        }
        self.left -= bytes;
        self.next += 1;
        Some(self.next)
    }
    fn give(&mut self, _p: usize, _off: usize, bytes: usize) -> bool {
        self.left += bytes;
        true
    }
}

fn budget(bytes: usize) -> Budget {
    Budget { left: bytes, next: 0 }
}

#[test]
fn quota_negata_non_apre() {
    let mut s = Surfaces::new();
    let mut q = budget(SURF_BYTES - 1);
    assert_eq!(s.open(&mut q, 0, 0, 0), Err(Err::Denied));
    assert!(s.pixels_mut(0).is_none());
}

#[test]
fn solo_il_proprio_buffer_e_scrivibile() {
    let mut s = Surfaces::new();
    let mut q = budget(SURF_BYTES * 2);
    s.open(&mut q, 1, 0, 0).unwrap();
    assert!(s.pixels_mut(0).is_none());
    let px = s.pixels_mut(1).unwrap();
    assert_eq!(px.len(), SPIX);
    px[0] = 42;
    assert_eq!(s.pixels_mut(1).unwrap()[0], 42);
}

#[test]
fn doppia_apertura_e_chiusura_restituisce_quota() {
    let mut s = Surfaces::new();
    let mut q = budget(SURF_BYTES);
    s.open(&mut q, 0, 0, 0).unwrap();
    assert_eq!(s.open(&mut q, 0, 5, 5), Err(Err::AlreadyOpen));
    s.close(&mut q, 0).unwrap();
    assert_eq!(q.left, SURF_BYTES);
    assert_eq!(s.close(&mut q, 0), Err(Err::NotOpen));
    assert_eq!(s.open(&mut q, 9, 0, 0), Err(Err::NoSuchProgram));
}

#[test]
fn compone_nell_ordine_dei_programmi() {
    let mut s = Surfaces::new();
    let mut q = budget(SURF_BYTES * 2);
    s.open(&mut q, 0, 0, 0).unwrap();
    s.open(&mut q, 1, 10, 10).unwrap();
    s.pixels_mut(0).unwrap().fill(1);
    s.pixels_mut(1).unwrap().fill(2);
    let mut fb = std::vec![0u16; 240 * 160];
    let n = s.compose_into(&mut fb);
    // Programma 0: 64x48 in (0,0). Programma 1: sopra, da (10,10).
    assert_eq!(n, 64 * 48 * 2);
    assert_eq!(fb[0], 1);
    assert_eq!(fb[10 * 240 + 10], 2);
    assert_eq!(fb[47 * 240 + 63], 2);
    assert_eq!(fb[60 * 240 + 0], 0);
}
