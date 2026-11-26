```rust
#![no_std]

/// Hash djb2 a 64 bit. L'overflow e' voluto: wrapping.
pub fn djb2(s: &[u8]) -> u64 {
    let mut h: u64 = 5381;
    for &c in s {
        h = h.wrapping_mul(33).wrapping_add(c as u64);
    }
    h
}
```
