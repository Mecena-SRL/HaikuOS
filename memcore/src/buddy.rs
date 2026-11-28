//! Allocatore buddy su bitmap fissa (riga 12).
//! Pagine numerate da 0. Un blocco di ordine k copre 2^k pagine e inizia a
//! `b << k`. Il bit b dell'ordine k vale 1 se quel blocco è libero.
//! `W` parole da 64 bit per ordine: capacità `W * 64` pagine, `O` ordini (0..O).
//! Richiede `W * 64` multiplo di `2^(O-1)`.

pub struct Buddy<const W: usize, const O: usize> {
    free: [[u64; W]; O],
}

impl<const W: usize, const O: usize> Buddy<W, O> {
    pub const PAGES: usize = W * 64;

    /// Nessuna pagina libera: si popola con `release`.
    pub const fn new() -> Self {
        Self { free: [[0; W]; O] }
    }

    fn bit(&self, k: usize, b: usize) -> bool {
        self.free[k][b / 64] >> (b % 64) & 1 == 1
    }

    fn set(&mut self, k: usize, b: usize, v: bool) {
        let m = 1u64 << (b % 64);
        if v {
            self.free[k][b / 64] |= m;
        } else {
            self.free[k][b / 64] &= !m;
        }
    }

    /// Primo blocco libero di ordine k (bit oltre i blocchi reali non vengono mai impostati).
    fn first(&self, k: usize) -> Option<usize> {
        self.free[k]
            .iter()
            .enumerate()
            .find(|(_, w)| **w != 0)
            .map(|(i, w)| i * 64 + w.trailing_zeros() as usize)
    }

    /// Alloca un blocco di 2^order pagine. Restituisce l'indice della prima pagina.
    pub fn alloc(&mut self, order: usize) -> Option<usize> {
        let mut k = order;
        let mut b = loop {
            if k >= O {
                return None;
            }
            if let Some(b) = self.first(k) {
                break b;
            }
            k += 1;
        };
        self.set(k, b, false);
        // Spezza fino all'ordine richiesto: tiene la metà bassa, libera quella alta.
        while k > order {
            k -= 1;
            b *= 2;
            self.set(k, b + 1, true);
        }
        Some(b << order)
    }

    /// Restituisce un blocco di 2^order pagine, fondendolo con il compagno libero.
    pub fn free(&mut self, page: usize, order: usize) {
        let mut k = order;
        let mut b = page >> order;
        while k + 1 < O && self.bit(k, b ^ 1) {
            self.set(k, b ^ 1, false);
            b >>= 1;
            k += 1;
        }
        self.set(k, b, true);
    }

    /// Mette in pool `count` pagine a partire da `start` (es. regioni utili della mappa BIOS).
    pub fn release(&mut self, start: usize, count: usize) {
        for p in start..start + count {
            self.free(p, 0);
        }
    }

    /// Pagine libere in totale.
    pub fn free_pages(&self) -> usize {
        (0..O)
            .map(|k| {
                let n: usize = self.free[k].iter().map(|w| w.count_ones() as usize).sum();
                n << k
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::Buddy;

    // 64 pagine, ordini 0..=6 (blocco massimo 64 pagine).
    type B = Buddy<1, 7>;

    #[test]
    fn all_single_pages_then_empty() {
        let mut b = B::new();
        b.release(0, B::PAGES);
        assert_eq!(b.free_pages(), 64);
        let mut seen = [false; 64];
        for _ in 0..64 {
            let p = b.alloc(0).expect("pagina");
            assert!(!seen[p], "pagina doppia");
            seen[p] = true;
        }
        assert_eq!(b.alloc(0), None);
        assert_eq!(b.free_pages(), 0);
    }

    #[test]
    fn blocks_are_aligned() {
        let mut b = B::new();
        b.release(0, B::PAGES);
        let p = b.alloc(3).unwrap();
        assert_eq!(p % 8, 0);
        assert_eq!(b.free_pages(), 56);
    }

    #[test]
    fn free_merges_back_to_whole() {
        let mut b = B::new();
        b.release(0, B::PAGES);
        let a = b.alloc(0).unwrap();
        let c = b.alloc(2).unwrap();
        b.free(a, 0);
        b.free(c, 2);
        assert_eq!(b.free_pages(), 64);
        // Tutta la memoria di nuovo in un unico blocco di ordine 6.
        assert_eq!(b.alloc(6), Some(0));
    }

    #[test]
    fn partial_range_and_no_overflow() {
        let mut b = B::new();
        b.release(8, 16); // pagine 8..24: due blocchi da 8 (ordine 3), nessun blocco da 16
        assert_eq!(b.free_pages(), 16);
        assert_eq!(b.alloc(4), None);
        assert_eq!(b.alloc(3), Some(8));
        assert_eq!(b.alloc(3), Some(16));
        assert_eq!(b.alloc(0), None);
    }
}
