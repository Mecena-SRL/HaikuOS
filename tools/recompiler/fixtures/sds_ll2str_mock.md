```rust
#![no_std]

/// Scrive `value` in decimale in `buf` con NUL finale; ritorna la lunghezza senza NUL.
/// Se `buf` è troppo corto, l'indicizzazione va in panico (il C avrebbe UB).
pub fn ll2str(buf: &mut [u8], value: i64) -> usize {
    let neg = value < 0;
    let mut v: u64 = value.unsigned_abs();
    let mut p = 0;
    loop {
        buf[p] = b'0' + (v % 10) as u8;
        p += 1;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    if neg {
        buf[p] = b'-';
        p += 1;
    }
    buf[..p].reverse();
    buf[p] = 0;
    p
}
```
