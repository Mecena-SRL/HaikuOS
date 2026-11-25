//! Superfici dei programmi: ogni programma ha un buffer di pixel fisso.
//! Il buffer si può aprire solo con una quota concessa dal monolite (`Quota`).
//! Un programma scrive solo nel proprio buffer; il compositore compone tutti i buffer aperti.

use crate::compositor::{self, Surface, W};

pub const SW: usize = 64;
pub const SH: usize = 48;
pub const SPIX: usize = SW * SH;
pub const SURF_BYTES: usize = SPIX * 2;
pub const N: usize = 4;

/// Chi concede la memoria: il monolite nel ROM, un finto nei test host.
pub trait Quota {
    /// Prende `bytes` per il programma `p`. Ritorna l'offset o `None` se negato.
    fn take(&mut self, p: usize, bytes: usize) -> Option<usize>;
    /// Restituisce `bytes` a `off` per il programma `p`.
    fn give(&mut self, p: usize, off: usize, bytes: usize) -> bool;
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Err {
    NoSuchProgram,
    AlreadyOpen,
    NotOpen,
    Denied,
}

#[derive(Clone, Copy)]
struct Entry {
    x: i32,
    y: i32,
    off: usize,
}

pub struct Surfaces {
    pixels: [[u16; SPIX]; N],
    open: [Option<Entry>; N],
}

impl Surfaces {
    pub const fn new() -> Self {
        Surfaces { pixels: [[0; SPIX]; N], open: [None; N] }
    }

    /// Apre la superficie del programma `p` in posizione `(x, y)`, se la quota la concede.
    pub fn open<Q: Quota>(&mut self, q: &mut Q, p: usize, x: i32, y: i32) -> Result<(), Err> {
        if p >= N {
            return Err(Err::NoSuchProgram);
        }
        if self.open[p].is_some() {
            return Err(Err::AlreadyOpen);
        }
        let off = q.take(p, SURF_BYTES).ok_or(Err::Denied)?;
        self.open[p] = Some(Entry { x, y, off });
        Ok(())
    }

    /// Chiude la superficie e restituisce la quota.
    pub fn close<Q: Quota>(&mut self, q: &mut Q, p: usize) -> Result<(), Err> {
        let e = self.open.get(p).copied().flatten().ok_or(Err::NotOpen)?;
        q.give(p, e.off, SURF_BYTES);
        self.open[p] = None;
        Ok(())
    }

    /// Pixel del programma `p`, solo se la sua superficie è aperta.
    pub fn pixels_mut(&mut self, p: usize) -> Option<&mut [u16]> {
        match self.open.get(p) {
            Some(Some(_)) => Some(&mut self.pixels[p][..]),
            _ => None,
        }
    }

    /// Compone le superfici aperte in ordine di programma (il numero più alto sta sopra).
    pub fn compose_into(&self, fb: &mut [u16]) -> usize {
        let empty: &[u16] = &[];
        let mut list = [Surface { x: 0, y: 0, w: 0, h: 0, pixels: empty, key: None }; N];
        for p in 0..N {
            if let Some(e) = self.open[p] {
                list[p] = Surface { x: e.x, y: e.y, w: SW, h: SH, pixels: &self.pixels[p], key: None };
            }
        }
        let _ = W;
        compositor::compose(fb, &list)
    }
}
