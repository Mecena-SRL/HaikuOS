//! Testo segnaposto (Lorem): genera parole da una lista fissa, in modo deterministico dal seme.
//! Scrive in un buffer fisso, nessuna allocazione.

pub const PAROLE: [&str; 12] = [
    "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit", "sed", "do", "eiusmod", "tempor",
];

/// Riempie `out` con parole separate da spazio e chiuse da un punto, senza superare la lunghezza.
/// Ritorna i byte scritti (0 se il buffer è troppo piccolo per una parola).
pub fn genera(seme: u32, out: &mut [u8]) -> usize {
    let mut s = seme;
    let mut n = 0;
    loop {
        s = s.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let w = PAROLE[((s >> 16) as usize) % PAROLE.len()].as_bytes();
        let sep = if n == 0 { 0 } else { 1 };
        if n + sep + w.len() + 1 > out.len() {
            break;
        }
        if sep == 1 {
            out[n] = b' ';
            n += 1;
        }
        out[n..n + w.len()].copy_from_slice(w);
        n += w.len();
    }
    if n == 0 {
        return 0;
    }
    out[n] = b'.';
    n + 1
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sta_nel_buffer_e_chiude_con_punto() {
        let mut b = [0u8; 40];
        let n = genera(7, &mut b);
        assert!(n > 0 && n <= 40);
        assert_eq!(b[n - 1], b'.');
    }

    #[test]
    fn stesso_seme_stesso_testo_e_buffer_piccolo() {
        let (mut a, mut b) = ([0u8; 30], [0u8; 30]);
        assert_eq!(genera(3, &mut a), genera(3, &mut b));
        assert_eq!(a, b);
        assert_eq!(genera(3, &mut [0u8; 4]), 0);
    }
}
