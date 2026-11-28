//! Punto fisso 16.16 e seno tabellato. `no_std`, niente f32.

pub type Fx = i32;
pub const ONE: Fx = 1 << 16;
/// Un giro completo, in unità di angolo (256 passi) in 16.16.
pub const FULL: Fx = 256 << 16;

#[inline]
pub const fn int(i: i32) -> Fx {
    i << 16
}

#[inline]
pub fn mul(a: Fx, b: Fx) -> Fx {
    ((a as i64 * b as i64) >> 16) as Fx
}

#[inline]
pub fn div(a: Fx, b: Fx) -> Fx {
    (((a as i64) << 16) / b as i64) as Fx
}

/// Radice intera (floor).
pub fn isqrt(n: u64) -> u64 {
    let (mut num, mut res, mut bit) = (n, 0u64, 1u64 << 62);
    while bit > num {
        bit >>= 2;
    }
    while bit != 0 {
        if num >= res + bit {
            num -= res + bit;
            res = (res >> 1) + bit;
        } else {
            res >>= 1;
        }
        bit >>= 2;
    }
    res
}

/// Seno di un angolo in passi (256 = giro). Approssimazione di Bhaskara I.
const fn sin_fx(a: i32) -> Fx {
    let d = (a & 255) * 360 / 256;
    let (neg, d) = if d >= 180 { (true, d - 180) } else { (false, d) };
    let p = (d * (180 - d)) as i64;
    let v = (4 * p * 65536 / (40500 - p)) as i32;
    if neg {
        -v
    } else {
        v
    }
}

const fn build_sin() -> [Fx; 256] {
    let mut t = [0; 256];
    let mut i = 0;
    while i < 256 {
        t[i] = sin_fx(i as i32);
        i += 1;
    }
    t
}

pub static SIN: [Fx; 256] = build_sin();

/// Vettore unitario (cos, sin) dell'angolo `h` (16.16, passi di 256 per giro).
#[inline]
pub fn dir(h: Fx) -> (Fx, Fx) {
    let i = ((h >> 16) & 255) as usize;
    (SIN[(i + 64) & 255], SIN[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trig_table_is_sane() {
        assert_eq!(SIN[0], 0);
        assert_eq!(SIN[64], ONE);
        assert!((SIN[32] - 46341).abs() < 300); // sin 45°
    }

    #[test]
    fn sqrt_and_mul() {
        assert_eq!(isqrt(1 << 32), 1 << 16);
        assert_eq!(mul(int(3), int(4)), int(12));
    }
}
