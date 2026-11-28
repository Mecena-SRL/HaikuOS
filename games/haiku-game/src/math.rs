//! Matematica intera.

/// Radice quadrata intera (floor) di `n`, senza overflow su tutto `u64`.
pub fn isqrt(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    let n = n as u128;
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x as u64
}

/// Come `isqrt` per i64; valori <= 0 danno 0.
pub fn isqrt_i64(n: i64) -> i64 {
    if n <= 0 {
        0
    } else {
        isqrt(n as u64) as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_values_are_exact_floor() {
        for n in 0..10_000u64 {
            let r = isqrt(n);
            assert!(r * r <= n && (r + 1) * (r + 1) > n, "n={n} r={r}");
        }
    }

    #[test]
    fn edges() {
        assert_eq!(isqrt(u64::MAX), 4_294_967_295);
        assert_eq!(isqrt_i64(-5), 0);
        assert_eq!(isqrt_i64(i64::MAX), 3_037_000_499);
    }
}
