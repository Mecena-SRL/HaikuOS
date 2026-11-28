//! Dischi: spazi di archiviazione con occupazione. Logica pura, senza allocazioni.
//! Il chiamante passa byte usati e totali (per esempio sistema 16 KB, utente 16 KB).

use super::sysutil::{Line, Out};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Disco {
    pub nome: &'static [u8],
    pub usati: usize,
    pub totali: usize,
}

/// Percentuale occupata; 0 se la capacità è zero.
pub fn percento(d: &Disco) -> usize {
    if d.totali == 0 {
        0
    } else {
        d.usati.min(d.totali) * 100 / d.totali
    }
}

/// Riga 1: nome e percentuale. Riga 2: usati su totali in KB.
pub fn out(d: &Disco) -> Out {
    (
        Line::new().push(d.nome).push(b" ").num(percento(d)).push(b"%"),
        Line::new().num(d.usati / 1024).push(b"/").num(d.totali / 1024).push(b"K"),
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn percentuale_e_righe() {
        let d = Disco { nome: b"SISTEMA", usati: 4096, totali: 16384 };
        assert_eq!(percento(&d), 25);
        let (a, b) = out(&d);
        assert_eq!(a.as_bytes(), b"SISTEMA 25%");
        assert_eq!(b.as_bytes(), b"4/16K");
    }

    #[test]
    fn capacita_zero_non_divide() {
        let d = Disco { nome: b"X", usati: 5, totali: 0 };
        assert_eq!(percento(&d), 0);
    }
}
