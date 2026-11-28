//! Verifica di integrità di un pacchetto: SHA-256 del payload.

use sha2::{Digest, Sha256};

/// SHA-256 del payload in esadecimale minuscolo.
pub fn sha256_hex(payload: &[u8]) -> String {
    Sha256::digest(payload)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Vero solo se il payload corrisponde al digest atteso (confronto case-insensitive).
pub fn verify(payload: &[u8], expected_hex: &str) -> bool {
    sha256_hex(payload).eq_ignore_ascii_case(expected_hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_digest() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn detects_tampering() {
        let d = sha256_hex(b"abc");
        assert!(verify(b"abc", &d.to_uppercase()));
        assert!(!verify(b"abd", &d));
    }
}
