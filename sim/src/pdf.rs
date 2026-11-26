//! Testo di base da PDF: flussi di contenuto (FlateDecode o non compressi), operatori di testo.
//! Limiti: niente font con codifica propria (testo esadecimale `<…>` ignorato), niente object
//! stream, niente xref. Un flusso che non si decomprime viene saltato. Non verificato su PDF reali.

use crate::inflate::inflate;
use crate::office::Sink;
use crate::zip::Error;

fn find(h: &[u8], n: &[u8]) -> Option<usize> {
    h.windows(n.len()).position(|w| w == n)
}

/// Testo di tutti i flussi di contenuto. `scratch` riceve ogni flusso decompresso.
pub fn text(pdf: &[u8], s: &mut Sink, scratch: &mut [u8]) -> Result<(), Error> {
    let mut i = 0;
    while let Some(p) = find(&pdf[i..], b"stream") {
        let kw = i + p;
        if kw >= 3 && &pdf[kw - 3..kw] == b"end" {
            i = kw + 6;
            continue;
        }
        let mut st = kw + 6;
        if pdf.get(st) == Some(&b'\r') {
            st += 1;
        }
        if pdf.get(st) == Some(&b'\n') {
            st += 1;
        }
        let end = find(&pdf[st..], b"endstream").map_or(pdf.len(), |e| st + e);
        // Dizionario dell'oggetto: dal `obj` precedente a `stream` (non sconfinare nel flusso prima).
        let lo = kw.saturating_sub(512);
        let dict = match pdf[lo..kw].windows(3).rposition(|w| w == b"obj") {
            Some(o) => &pdf[lo + o..kw],
            None => &pdf[lo..kw],
        };
        let data = &pdf[st..end];
        i = end + 9;
        let n = if find(dict, b"FlateDecode").is_some() {
            match data.get(2..).map(|d| inflate(d, scratch)) {
                Some(Ok(n)) => n,
                Some(Err(Error::Full)) => return Err(Error::Full),
                _ => continue,
            }
        } else {
            if data.len() > scratch.len() {
                return Err(Error::Full);
            }
            scratch[..data.len()].copy_from_slice(data);
            data.len()
        };
        content(&scratch[..n], s)?;
    }
    Ok(())
}

/// Operatori: `Td`, `TD`, `T*`, `Tm`, `ET`, `'` vanno a capo. Un numero ≤ -200 in un `TJ` è uno spazio.
fn content(c: &[u8], s: &mut Sink) -> Result<(), Error> {
    let mut i = 0;
    while i < c.len() {
        let b = c[i];
        if b == b'(' {
            i = string(c, i, s)?;
        } else if b == b'%' {
            while i < c.len() && c[i] != b'\n' && c[i] != b'\r' {
                i += 1;
            }
        } else if b == b'-' || b.is_ascii_digit() {
            let neg = b == b'-';
            if neg {
                i += 1;
            }
            let st = i;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == b'.') {
                i += 1;
            }
            let int = c[st..i].split(|&x| x == b'.').next().unwrap_or(&[]);
            let v = int.iter().fold(0usize, |a, &d| a.saturating_mul(10).saturating_add((d - b'0') as usize));
            if neg && v >= 200 {
                s.byte(b' ')?;
            }
        } else if b.is_ascii_alphabetic() || b == b'*' || b == b'\'' {
            let st = i;
            while i < c.len() && (c[i].is_ascii_alphanumeric() || c[i] == b'*' || c[i] == b'\'') {
                i += 1;
            }
            match &c[st..i] {
                b"Td" | b"TD" | b"T*" | b"Tm" | b"ET" | b"'" => s.nl()?,
                _ => {}
            }
        } else {
            i += 1;
        }
    }
    Ok(())
}

/// Stringa `( … )` a partire da `i` (che punta a `(`). Restituisce la posizione dopo `)`.
fn string(c: &[u8], mut i: usize, s: &mut Sink) -> Result<usize, Error> {
    let mut depth = 0usize;
    while i < c.len() {
        let b = c[i];
        i += 1;
        match b {
            b'(' => {
                depth += 1;
                if depth > 1 {
                    s.push_char(b as char)?;
                }
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
                s.push_char(')')?;
            }
            b'\\' => {
                let e = *c.get(i).unwrap_or(&b'\\');
                i += 1;
                match e {
                    b'n' => s.byte(b'\n')?,
                    b'r' => s.byte(b'\r')?,
                    b't' => s.byte(b'\t')?,
                    b'0'..=b'7' => {
                        let mut v = (e - b'0') as u32;
                        for _ in 0..2 {
                            match c.get(i) {
                                Some(&d @ b'0'..=b'7') => {
                                    v = v * 8 + (d - b'0') as u32;
                                    i += 1;
                                }
                                _ => break,
                            }
                        }
                        s.push_char(char::from_u32(v).unwrap_or('?'))?;
                    }
                    _ => s.push_char(e as char)?,
                }
            }
            // Codifica WinAnsi/Latin-1 approssimata: ogni byte è un carattere.
            _ => s.push_char(b as char)?,
        }
    }
    Ok(i)
}

