//! Distruggi file: sovrascrive i byte con tre schemi e infine con zeri, prima della cancellazione.
//! Logica pura su slice. Nota: su memoria a usura livellata (SRAM, flash) la garanzia è limitata: da verificare.

pub const SCHEMI: [u8; 3] = [0x00, 0xFF, 0x55];

/// Sovrascrive `buf` con ogni schema, poi con zeri. Ritorna le passate eseguite (4).
pub fn distruggi(buf: &mut [u8]) -> u8 {
    for s in SCHEMI {
        buf.fill(s);
    }
    buf.fill(0);
    SCHEMI.len() as u8 + 1
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn azzera_tutto_il_buffer() {
        let mut b = [0xAAu8; 16];
        assert_eq!(distruggi(&mut b), 4);
        assert!(b.iter().all(|&x| x == 0));
    }
}
