```rust
#![no_std]

pub fn djb2(s: &[u8]) -> u64 {
    let mut h: u64 = 5381;
    for &c in s {
        h = h * 33 + c as u64;
    }
    h
}
```
