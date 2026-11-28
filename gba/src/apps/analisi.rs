//! Analizzatore utilizzo disco: classifica dei file più grandi e barra di occupazione.
//! Logica pura, senza allocazioni. Le voci sono (nome, blocchi): il chiamante le legge da `Fs`.

use super::sysutil::{Line, Out};

/// Quanti file mostrare in classifica.
pub const TOP: usize = 3;

/// Scrive in `out` gli indici dei `TOP` elementi più grandi, dal maggiore (a parità vince il primo).
/// Restituisce quanti indici sono validi.
pub fn classifica(voci: &[(&[u8], usize)], out: &mut [usize; TOP]) -> usize {
    let mut n = 0;
    for (i, v) in voci.iter().enumerate() {
        let mut pos = n;
        while pos > 0 && voci[out[pos - 1]].1 < v.1 {
            pos -= 1;
        }
        if pos < TOP {
            let mut k = if n < TOP { n } else { TOP - 1 };
            while k > pos {
                out[k] = out[k - 1];
                k -= 1;
            }
            out[pos] = i;
            if n < TOP {
                n += 1;
            }
        }
    }
    n
}

/// Barra di 10 caselle, `[#####.....]`: quota di `usati` su `totali` blocchi.
pub fn barra(usati: usize, totali: usize) -> Line {
    let pieni = if totali == 0 { 0 } else { usati.min(totali) * 10 / totali };
    let mut l = Line::new().push(b"[");
    for i in 0..10 {
        let c: &[u8] = if i < pieni { b"#" } else { b"." };
        l = l.push(c);
    }
    l.push(b"]")
}

/// Schermata: barra, poi i file più grandi come `nome KB`.
pub fn schermata(voci: &[(&[u8], usize)], blocco: usize, totali: usize) -> [Line; TOP + 1] {
    let usati: usize = voci.iter().map(|v| v.1).sum();
    let mut top = [0usize; TOP];
    let n = classifica(voci, &mut top);
    let mut r = [Line::new(); TOP + 1];
    r[0] = barra(usati, totali);
    for k in 0..n {
        let (nome, blocchi) = voci[top[k]];
        r[k + 1] = Line::new().push(nome).push(b" ").num(blocchi * blocco / 1024).push(b"K");
    }
    r
}

/// Riepilogo in due righe: blocchi usati su totali e numero di file.
pub fn out(voci: &[(&[u8], usize)], totali: usize) -> Out {
    let usati: usize = voci.iter().map(|v| v.1).sum();
    (Line::new().push(b"USO ").num(usati).push(b"/").num(totali), Line::new().push(b"FILE ").num(voci.len()))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn classifica_dal_maggiore() {
        let v: [(&[u8], usize); 5] = [(b"a", 2), (b"b", 9), (b"c", 4), (b"d", 9), (b"e", 1)];
        let mut top = [0usize; TOP];
        let n = classifica(&v, &mut top);
        assert_eq!(n, 3);
        assert_eq!(top, [1, 3, 2]); // b e d a pari: prima b
    }

    #[test]
    fn meno_file_del_massimo() {
        let v: [(&[u8], usize); 1] = [(b"solo", 7)];
        let mut top = [0usize; TOP];
        assert_eq!(classifica(&v, &mut top), 1);
        assert_eq!(top[0], 0);
    }

    #[test]
    fn barra_proporzionale() {
        assert_eq!(barra(50, 100).as_bytes(), b"[#####.....]");
        assert_eq!(barra(0, 0).as_bytes(), b"[..........]");
    }

    #[test]
    fn schermata_ordinata() {
        let v: [(&[u8], usize); 2] = [(b"piccolo", 2), (b"grande", 8)];
        let r = schermata(&v, 64, 16);
        assert_eq!(r[0].as_bytes(), b"[######....]"); // 10 su 16 blocchi
        assert_eq!(r[1].as_bytes(), b"grande 0K");
        assert_eq!(r[2].as_bytes(), b"piccolo 0K");
        assert!(r[3].is_empty());
    }
}
