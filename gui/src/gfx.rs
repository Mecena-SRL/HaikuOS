//! Primitive a 32 bit su pixel fisici: ritaglio, riempimento, copia e lista fissa di zone sporche.
//! no_std, nessuna allocazione: il chiamante passa slice e stride.
use crate::layout::Rect;

/// Ritaglia `r` sull'area `w`×`h`. `None` se il risultato è vuoto.
pub fn clip(r: Rect, w: i32, h: i32) -> Option<Rect> {
    let (x0, y0) = (r.x.max(0), r.y.max(0));
    let (x1, y1) = ((r.x + r.w).min(w), (r.y + r.h).min(h));
    if x1 <= x0 || y1 <= y0 {
        None
    } else {
        Some(Rect::new(x0, y0, x1 - x0, y1 - y0))
    }
}

/// Intersezione di `a` e `b`. `None` se è vuota.
pub fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    let (x0, y0) = (a.x.max(b.x), a.y.max(b.y));
    let (x1, y1) = ((a.x + a.w).min(b.x + b.w), (a.y + a.h).min(b.y + b.h));
    if x1 <= x0 || y1 <= y0 {
        None
    } else {
        Some(Rect::new(x0, y0, x1 - x0, y1 - y0))
    }
}

/// Riempie `r` (già ritagliato) nel buffer di stride `stride`: una `fill` per riga, senza ciclo per pixel.
pub fn fill(buf: &mut [u32], stride: usize, r: Rect, c: u32) {
    let (x0, w) = (r.x as usize, r.w as usize);
    for y in r.y as usize..(r.y + r.h) as usize {
        let o = y * stride + x0;
        buf[o..o + w].fill(c);
    }
}

/// Copia `r` (già ritagliato) da `src` a `dst`, stesso stride: una `copy_from_slice` per riga.
pub fn copy(dst: &mut [u32], src: &[u32], stride: usize, r: Rect) {
    let (x0, w) = (r.x as usize, r.w as usize);
    for y in r.y as usize..(r.y + r.h) as usize {
        let o = y * stride + x0;
        dst[o..o + w].copy_from_slice(&src[o..o + w]);
    }
}

/// Lista fissa di zone sporche. Se si riempie, le fonde tutte in un'unica zona (il bounding box):
/// il costo resta limitato e non si perde mai un cambiamento.
#[derive(Clone, Copy, Debug)]
pub struct Dirty<const N: usize> {
    rects: [Rect; N],
    len: usize,
}

impl<const N: usize> Dirty<N> {
    pub const fn new() -> Self {
        Dirty { rects: [Rect::new(0, 0, 0, 0); N], len: 0 }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn rects(&self) -> &[Rect] {
        &self.rects[..self.len]
    }

    /// Segna `r` come sporco. Fonde con le zone che si sovrappongono.
    pub fn add(&mut self, r: Rect) {
        if r.w <= 0 || r.h <= 0 {
            return;
        }
        let mut r = r;
        let mut i = 0;
        while i < self.len {
            if overlaps(self.rects[i], r) {
                r = union(self.rects[i], r);
                self.len -= 1;
                self.rects[i] = self.rects[self.len];
                i = 0; // l'unione può toccarne altre: ricontrolla dall'inizio
            } else {
                i += 1;
            }
        }
        if self.len == N {
            let mut all = self.rects[0];
            for k in 1..N {
                all = union(all, self.rects[k]);
            }
            self.rects[0] = union(all, r);
            self.len = 1;
            return;
        }
        self.rects[self.len] = r;
        self.len += 1;
    }
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

fn union(a: Rect, b: Rect) -> Rect {
    let x0 = a.x.min(b.x);
    let y0 = a.y.min(b.y);
    let x1 = (a.x + a.w).max(b.x + b.w);
    let y1 = (a.y + a.h).max(b.y + b.h);
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}
