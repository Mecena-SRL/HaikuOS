//! Hash delle password: PBKDF2-HMAC-SHA256 con sale casuale per utente.

use crate::text::Text;
use sha2::Sha256;

/// Iterazioni di default (raccomandazione OWASP 2023 per PBKDF2-SHA256).
pub const ITERATIONS: u32 = 600_000;
pub const SALT_LEN: usize = 16;
pub const DIGEST_LEN: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Hash {
    pub iterations: u32,
    pub salt: [u8; SALT_LEN],
    pub digest: [u8; DIGEST_LEN],
}

impl Hash {
    pub fn new(password: &[u8], salt: &[u8; SALT_LEN], iterations: u32) -> Self {
        let mut digest = [0u8; DIGEST_LEN];
        pbkdf2::pbkdf2_hmac::<Sha256>(password, salt, iterations, &mut digest);
        Self { iterations, salt: *salt, digest }
    }

    /// Confronto a tempo costante con la password fornita.
    pub fn verify(&self, password: &[u8]) -> bool {
        let other = Hash::new(password, &self.salt, self.iterations);
        let mut diff = 0u8;
        for (a, b) in self.digest.iter().zip(other.digest.iter()) {
            diff |= a ^ b;
        }
        diff == 0
    }

    /// Formato: `pbkdf2-sha256$<iterazioni>$<sale hex>$<digest hex>`.
    pub fn encode(&self) -> Text<128> {
        let mut out = [0u8; 128];
        let mut n = 0;
        let mut put = |bytes: &[u8]| {
            out[n..n + bytes.len()].copy_from_slice(bytes);
            n += bytes.len();
        };
        put(b"pbkdf2-sha256$");
        let mut num = [0u8; 10];
        let mut i = num.len();
        let mut v = self.iterations;
        loop {
            i -= 1;
            num[i] = b'0' + (v % 10) as u8;
            v /= 10;
            if v == 0 {
                break;
            }
        }
        put(&num[i..]);
        put(b"$");
        let mut hex = [0u8; 2];
        for b in self.salt.iter() {
            hex_byte(*b, &mut hex);
            put(&hex);
        }
        put(b"$");
        for b in self.digest.iter() {
            hex_byte(*b, &mut hex);
            put(&hex);
        }
        Text::from_bytes(&out[..n]).unwrap_or_else(Text::empty)
    }
}

fn hex_byte(b: u8, out: &mut [u8; 2]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out[0] = HEX[(b >> 4) as usize];
    out[1] = HEX[(b & 0xf) as usize];
}
