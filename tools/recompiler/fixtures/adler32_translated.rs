#![no_std]

const BASE: u64 = 65521;

/// Adler-32 di zlib: somme a e b modulo BASE, poi impacchettate in 32 bit.
pub fn adler32(adler: u64, buf: &[u8]) -> u64 {
    let mut a = adler & 0xffff;
    let mut b = (adler >> 16) & 0xffff;
    for &x in buf {
        a = (a + x as u64) % BASE;
        b = (b + a) % BASE;
    }
    (b << 16) | a
}
