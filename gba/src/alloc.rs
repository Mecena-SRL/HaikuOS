//! Allocatore a blocchi fissi per slot: bitmap, prima corrispondenza su run contigui.
//! Dimensione del blocco e numero di blocchi sono parametri (qui: 64 byte, 512 blocchi = 32 KB).
#![allow(dead_code)]

pub const BLK: usize = 64;
pub const ARENA_BLOCKS: usize = 512;
const WORDS: usize = ARENA_BLOCKS / 32;

#[derive(Clone, Copy)]
pub struct Arena {
    bits: [u32; WORDS],
}

impl Arena {
    pub const fn new() -> Self {
        Self { bits: [0; WORDS] }
    }

    fn is_set(&self, i: usize) -> bool {
        self.bits[i / 32] & (1 << (i % 32)) != 0
    }

    fn set(&mut self, i: usize, v: bool) {
        if v {
            self.bits[i / 32] |= 1 << (i % 32);
        } else {
            self.bits[i / 32] &= !(1 << (i % 32));
        }
    }

    pub fn used_blocks(&self) -> usize {
        self.bits.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// Alloca `bytes` (arrotondati su blocchi). Ritorna l'offset in byte nell'arena.
    pub fn alloc(&mut self, bytes: usize) -> Option<usize> {
        let n = bytes.div_ceil(BLK).max(1);
        if n > ARENA_BLOCKS {
            return None;
        }
        let mut start = 0;
        while start + n <= ARENA_BLOCKS {
            match (start..start + n).find(|&i| self.is_set(i)) {
                None => {
                    for i in start..start + n {
                        self.set(i, true);
                    }
                    return Some(start * BLK);
                }
                Some(hit) => start = hit + 1,
            }
        }
        None
    }

    /// Libera un'allocazione fatta con `alloc(bytes)` all'offset `off`.
    pub fn free(&mut self, off: usize, bytes: usize) -> bool {
        let n = bytes.div_ceil(BLK).max(1);
        let s = off / BLK;
        if off % BLK != 0 || s + n > ARENA_BLOCKS || !(s..s + n).all(|i| self.is_set(i)) {
            return false;
        }
        for i in s..s + n {
            self.set(i, false);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_and_free() {
        let mut a = Arena::new();
        let x = a.alloc(100).unwrap(); // 2 blocchi
        let y = a.alloc(64).unwrap(); // 1 blocco
        assert_eq!((x, y), (0, 128));
        assert_eq!(a.used_blocks(), 3);
        assert!(a.free(x, 100));
        assert!(!a.free(x, 100)); // doppia liberazione rifiutata
        assert_eq!(a.alloc(128), Some(0)); // riusa il buco
    }

    #[test]
    fn full_arena_refuses() {
        let mut a = Arena::new();
        assert_eq!(a.alloc(ARENA_BLOCKS * BLK), Some(0));
        assert_eq!(a.alloc(1), None);
        assert_eq!(a.alloc(ARENA_BLOCKS * BLK + 1), None);
    }
}
