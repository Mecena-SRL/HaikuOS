//! Fogli ODS in sola lettura (`content.xml`): celle a tabulazione, righe a LF. Solo `core`.
//! Ogni foglio inizia col suo nome. Celle ripetute: testo una volta, tabulazioni per tutte.
//! XLSX è in `sheet.rs`.

use crate::formats::{push, tag, text};

/// Valore dell'attributo `key="…"` nel tag `t` (gli attributi sono separati da spazio).
fn attr<'a>(t: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let n = key.len();
    let p = t.windows(n + 3).position(|w| w[0] == b' ' && &w[1..1 + n] == key && &w[1 + n..] == b"=\"")?;
    let st = p + n + 3;
    let len = t[st..].iter().position(|&c| c == b'"')?;
    Some(&t[st..st + len])
}

fn num(v: Option<&[u8]>) -> usize {
    v.map_or(0, |v| v.iter().filter(|c| c.is_ascii_digit()).fold(0, |a, &c| a * 10 + (c - b'0') as usize))
}

/// Toglie le tabulazioni finali (celle vuote) e va a capo.
fn newline(out: &mut [u8], o: &mut usize) -> Option<()> {
    while *o > 0 && out[*o - 1] == b'\t' {
        *o -= 1;
    }
    push(out, o, b'\n')
}

fn tabs(out: &mut [u8], o: &mut usize, k: usize) -> Option<()> {
    for _ in 0..k.min(256) {
        push(out, o, b'\t')?;
    }
    Some(())
}

/// Stringa n. `idx` di `sst`: scrive solo il testo dei `<t>` del paragrafo, saltando `rPh`.
fn shared(sst: &[u8], idx: usize, out: &mut [u8], o: &mut usize) -> Option<()> {
    let (mut i, mut k, mut active, mut in_t, mut skip) = (0usize, 0usize, false, false, false);
    while i < sst.len() {
        if sst[i] != b'<' {
            let next = i + sst[i..].iter().position(|&b| b == b'<').unwrap_or(sst.len() - i);
            if active && in_t && !skip {
                text(&sst[i..next], out, o)?;
            }
            i = next;
            continue;
        }
        let (name, close, selfclose, nx) = tag(sst, i)?;
        match name {
            b"si" if !close => active = k == idx,
            b"si" if close => {
                if active {
                    return Some(());
                }
                k += 1;
            }
            b"rPh" => skip = !close && !selfclose,
            b"t" => in_t = !close && !selfclose,
            _ => {}
        }
        i = nx;
    }
    Some(())
}

/// Fogli ODS (`content.xml`). Ogni foglio inizia col suo nome. Celle ripetute: testo una volta.
pub fn read_ods(xml: &[u8], out: &mut [u8]) -> Option<usize> {
    let (mut o, mut i, mut rep) = (0usize, 0usize, 1usize);
    while i < xml.len() {
        if xml[i] != b'<' {
            let next = i + xml[i..].iter().position(|&b| b == b'<').unwrap_or(xml.len() - i);
            let s = &xml[i..next];
            if !s.iter().all(|c| c.is_ascii_whitespace()) {
                text(s, out, &mut o)?;
            }
            i = next;
            continue;
        }
        let (name, close, selfclose, nx) = tag(xml, i)?;
        let t = &xml[i..nx];
        match name {
            b"table:table" if !close => {
                if let Some(n) = attr(t, b"table:name") {
                    text(n, out, &mut o)?;
                    push(out, &mut o, b'\n')?;
                }
            }
            b"table:table-cell" | b"table:covered-table-cell" => {
                if close {
                    tabs(out, &mut o, rep)?;
                } else {
                    rep = num(attr(t, b"table:number-columns-repeated")).max(1);
                    if selfclose {
                        tabs(out, &mut o, rep)?;
                    }
                }
            }
            b"table:table-row" if close || selfclose => newline(out, &mut o)?,
            _ => {}
        }
        i = nx;
    }
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ods_fogli_righe_e_celle_ripetute() {
        let xml = b"<office:document-content><table:table table:name=\"Foglio1\"><table:table-row><table:table-cell office:value-type=\"float\"><text:p>3</text:p></table:table-cell><table:table-cell table:number-columns-repeated=\"2\"/><table:table-cell><text:p>Ok</text:p></table:table-cell></table:table-row><table:table-row><table:table-cell><text:p>a</text:p></table:table-cell><table:table-cell><text:p>b</text:p></table:table-cell></table:table-row></table:table></office:document-content>";
        let mut out = [0u8; 128];
        let n = read_ods(xml, &mut out).unwrap();
        assert_eq!(&out[..n], "Foglio1\n3\t\t\tOk\na\tb\n".as_bytes());
    }
}
