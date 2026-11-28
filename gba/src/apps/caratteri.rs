//! Caratteri: riepilogo dei font installati e attivi. Logica pura: i conteggi li passa il chiamante da `fontmgr`.

use super::sysutil::{Line, Out};

/// Riga 1: installati su massimo. Riga 2: attivi su massimo attivi.
pub fn out(installati: usize, massimo: usize, attivi: usize, max_attivi: usize) -> Out {
    (
        Line::new().push(b"CARATTERI ").num(installati).push(b"/").num(massimo),
        Line::new().push(b"ATTIVI ").num(attivi).push(b"/").num(max_attivi),
    )
}

/// Vero se non si può installare un altro font.
pub fn pieno(installati: usize, massimo: usize) -> bool {
    installati >= massimo
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn riepilogo_e_pieno() {
        let (a, b) = out(2, 4, 1, 4);
        assert_eq!(a.as_bytes(), b"CARATTERI 2/4");
        assert_eq!(b.as_bytes(), b"ATTIVI 1/4");
        assert!(!pieno(3, 4));
        assert!(pieno(4, 4));
    }
}
