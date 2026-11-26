//! Presentazioni: formato nativo di testo (diapositive separate da una riga `---`).
//! Lettura di pptx: testo delle diapositive, in ordine di numero. Scrittura pptx: **rinviata**
//! (servono tema e master validi; da verificare con LibreOffice e PowerPoint).
//! Solo `core`, nessuna allocazione.

use crate::formats::{tag, text};
use crate::zip;

/// Numero di diapositive del testo nativo (una riga `---` separa due diapositive).
pub fn count(src: &[u8]) -> usize {
    1 + src.split(|&b| b == b'\n').filter(|l| *l == b"---").count()
}

/// Testo della diapositiva `idx`, senza le righe separatrici.
pub fn slide(src: &[u8], idx: usize) -> Option<&[u8]> {
    let mut start = 0;
    for n in 0..=idx {
        let mut p = start;
        loop {
            let e = src[p..].iter().position(|&b| b == b'\n').map_or(src.len(), |q| p + q);
            if &src[p..e] == b"---" {
                let end = if p > start { p - 1 } else { start };
                if n == idx {
                    return Some(&src[start..end]);
                }
                start = e + 1;
                break;
            }
            if e >= src.len() {
                return if n == idx { Some(&src[start..]) } else { None };
            }
            p = e + 1;
        }
    }
    None
}

/// Legge le diapositive di un pptx e le scrive nel formato nativo (`---` tra una e l'altra).
/// `scratch` riceve ogni file XML delle diapositive.
pub fn read_pptx(pptx: &[u8], scratch: &mut [u8], out: &mut [u8]) -> Option<usize> {
    let mut o = 0;
    let mut k = 1;
    let mut name = [0u8; 32];
    while let Some(n) = slide_name(k, &mut name) {
        let Some(len) = zip::extract(pptx, &name[..n], scratch) else { break };
        if k > 1 {
            for &b in b"\n---\n" {
                crate::formats::push(out, &mut o, b)?;
            }
        }
        let xml = &scratch[..len];
        let mut i = 0;
        let mut para = 0;
        let mut in_t = false;
        while i < xml.len() {
            if xml[i] != b'<' {
                let next = i + xml[i..].iter().position(|&b| b == b'<').unwrap_or(xml.len() - i);
                if in_t {
                    text(&xml[i..next], out, &mut o)?;
                }
                i = next;
                continue;
            }
            let (name, close, selfclose, next) = tag(xml, i)?;
            match name {
                b"a:p" if !close && !selfclose => {
                    if para > 0 {
                        crate::formats::push(out, &mut o, b'\n')?;
                    }
                    para += 1;
                }
                b"a:t" if !selfclose => in_t = !close,
                _ => {}
            }
            i = next;
        }
        k += 1;
    }
    if k == 1 {
        return None;
    }
    Some(o)
}

/// "ppt/slides/slideK.xml" in `buf`. Ritorna la lunghezza.
fn slide_name(k: usize, buf: &mut [u8; 32]) -> Option<usize> {
    if k > 999 {
        return None;
    }
    let head = b"ppt/slides/slide";
    buf[..head.len()].copy_from_slice(head);
    let mut o = head.len();
    let mut digits = [0u8; 4];
    let mut d = 0;
    let mut v = k;
    loop {
        digits[d] = b'0' + (v % 10) as u8;
        d += 1;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    while d > 0 {
        d -= 1;
        buf[o] = digits[d];
        o += 1;
    }
    for &b in b".xml" {
        buf[o] = b;
        o += 1;
    }
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diapositive_nativo() {
        let src = b"Titolo\nriga\n---\nSecondo\n---\nTerzo";
        assert_eq!(count(src), 3);
        assert_eq!(slide(src, 0), Some(&b"Titolo\nriga"[..]));
        assert_eq!(slide(src, 1), Some(&b"Secondo"[..]));
        assert_eq!(slide(src, 2), Some(&b"Terzo"[..]));
        assert_eq!(slide(src, 3), None);
    }

    #[test]
    fn pptx_di_pandoc() {
        let mut scratch = [0u8; 8192];
        let mut out = [0u8; 1024];
        let n = read_pptx(include_bytes!("../tests/fixtures/sample.pptx"), &mut scratch, &mut out).unwrap();
        assert_eq!(&out[..n], "Titolo\nriga uno\nriga due\n---\nSecondo\nTesto & fine".as_bytes());
    }
}
