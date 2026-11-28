//! Generatore xorshift32: deterministico, seme 0 diventa 1.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rng(u32);

impl Rng {
    pub const fn new(seed: u32) -> Self {
        Rng(if seed == 0 { 1 } else { seed })
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// Valore in `0..m` (m > 0).
    pub fn below(&mut self, m: u32) -> u32 {
        self.next_u32() % m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_seed_is_fixed_and_never_stuck() {
        let mut r = Rng::new(0);
        assert_eq!(r, Rng::new(1));
        assert!((0..100).all(|_| r.next_u32() != 0));
    }

    #[test]
    fn same_seed_same_stream() {
        let (mut a, mut b) = (Rng::new(42), Rng::new(42));
        for _ in 0..1000 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }
}
