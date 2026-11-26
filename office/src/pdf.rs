//! PDF in sola lettura: testo dei flussi di contenuto.
//! Font semplici (WinAnsi, byte diretti) e font Identity-H con ToUnicode (2 byte per glifo).
//! Limiti (**da verificare**): niente cifratura, niente xref (si scandisce il file), CMap solo nei
//! formati bfchar/bfrange. Solo `core`, nessuna allocazione: `scratch` e `cbuf` tengono i flussi decompressi.

use crate::inflate::inflate;

fn find(h: &[u8], pat: &[u8], from: usize) -> Option<usize> {
    if from >= h.len() {
        return None;
    }
    h[from..].windows(pat.len()).position(|w| w == pat).map(|p| p + from)
}

fn is_delim(b: u8) -> bool {
    matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%') || b.is_ascii_whitespace()
}

fn skip_ws(h: &[u8], mut i: usize) -> usize {
    while i < h.len() && h[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// Legge "N 0 R" a partire da `i`, ritorna N.
fn parse_ref(h: &[u8], i: usize) -> Option<usize> {
    let mut i = skip_ws(h, i);
    let s = i;
    while i < h.len() && h[i].is_ascii_digit() {
        i += 1;
    }
    if i == s {
        return None;
    }
    let n = h[s..i].iter().fold(0usize, |a, &d| a * 10 + (d - b'0') as usize);
    let i = skip_ws(h, i);
    if h.get(i..i + 1)? != b"0" {
        return None;
    }
    let i = skip_ws(h, i + 1);
    (h.get(i) == Some(&b'R')).then_some(n)
}

/// Corpo dell'oggetto `n` (tra "n 0 obj" e "endobj").
fn obj_body(pdf: &[u8], n: usize) -> Option<&[u8]> {
    let mut digits = [0u8; 20];
    let mut k = digits.len();
    let mut v = n;
    loop {
        k -= 1;
        digits[k] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    let head = &digits[k..];
    let mut from = 0;
    while let Some(p) = find(pdf, head, from) {
        from = p + 1;
        let before_ok = p == 0 || pdf[p - 1].is_ascii_whitespace();
        let rest = &pdf[p + head.len()..];
        if before_ok && rest.starts_with(b" 0 obj") {
            let start = p + head.len() + 6;
            let end = find(pdf, b"endobj", start)?;
            return Some(&pdf[start..end]);
        }
    }
    None
}

/// Dizionario e dati di un flusso che inizia alla posizione `s` (parola "stream").
fn stream_at(h: &[u8], s: usize) -> Option<(&[u8], &[u8])> {
    let obj = s.saturating_sub(4096);
    let dict_start = h[obj..s].windows(3).rposition(|w| w == b"obj").map(|p| p + obj).unwrap_or(obj);
    let mut ds = s + 6;
    if h.get(ds) == Some(&b'\r') {
        ds += 1;
    }
    if h.get(ds) == Some(&b'\n') {
        ds += 1;
    }
    let de = find(h, b"endstream", ds)?;
    Some((&h[dict_start..s], &h[ds..de]))
}

/// Decomprime un flusso (FlateDecode o non compresso) in `dst`.
fn decode(dict: &[u8], data: &[u8], dst: &mut [u8]) -> Option<usize> {
    if find(dict, b"FlateDecode", 0).is_some() {
        // In PDF il FlateDecode è zlib: 2 byte di intestazione prima del DEFLATE.
        inflate(data.get(2..)?, dst)
    } else {
        dst.get_mut(..data.len())?.copy_from_slice(data);
        Some(data.len())
    }
}

fn push(out: &mut [u8], o: &mut usize, b: u8) -> Option<()> {
    *out.get_mut(*o)? = b;
    *o += 1;
    Some(())
}

fn push_char(out: &mut [u8], o: &mut usize, cp: u32) -> Option<()> {
    let c = char::from_u32(cp).unwrap_or('?');
    let mut tmp = [0u8; 4];
    for &b in c.encode_utf8(&mut tmp).as_bytes() {
        push(out, o, b)?;
    }
    Some(())
}

fn newline(out: &mut [u8], o: &mut usize) -> Option<()> {
    if *o > 0 && out[*o - 1] != b'\n' {
        push(out, o, b'\n')?;
    }
    Some(())
}

/// Prossimo token di una CMap: stringa esadecimale `<..>` o parola.
fn token<'a>(cm: &'a [u8], i: &mut usize) -> Option<&'a [u8]> {
    *i = skip_ws(cm, *i);
    let s = *i;
    if *i >= cm.len() {
        return None;
    }
    if cm[s] == b'<' {
        let e = s + cm[s..].iter().position(|&b| b == b'>')?;
        *i = e + 1;
        return Some(&cm[s..=e]);
    }
    if is_delim(cm[s]) {
        *i += 1;
        return Some(&cm[s..s + 1]);
    }
    while *i < cm.len() && !is_delim(cm[*i]) {
        *i += 1;
    }
    Some(&cm[s..*i])
}

/// Valore esadecimale di `<..>`, e numero di byte.
fn hex_val(tok: &[u8]) -> (u32, usize) {
    let mut v = 0u32;
    let mut n: usize = 0;
    for &b in tok.iter().skip(1) {
        let d = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => continue,
        };
        // Ogni coppia di cifre è un byte.
        v = v.wrapping_mul(16).wrapping_add(d as u32);
        n += 1;
    }
    (v, n.div_ceil(2))
}

/// Codice a 2 byte -> valore Unicode (UTF-16BE) e numero di byte del valore.
fn cmap_lookup(cm: &[u8], code: u32) -> Option<(u32, usize)> {
    let mut i = 0;
    let mut mode = 0u8; // 1 = bfchar, 2 = bfrange
    while let Some(t) = token(cm, &mut i) {
        match t {
            b"beginbfchar" => mode = 1,
            b"beginbfrange" => mode = 2,
            b"endbfchar" | b"endbfrange" => mode = 0,
            _ if mode == 1 => {
                let dst = token(cm, &mut i)?;
                if t.first() == Some(&b'<') && hex_val(t).0 == code {
                    return Some(hex_val(dst));
                }
            }
            _ if mode == 2 => {
                let hi_t = token(cm, &mut i)?;
                let dst = token(cm, &mut i)?;
                let (lo, _) = hex_val(t);
                let (hi, _) = hex_val(hi_t);
                if t.first() == Some(&b'<') && (lo..=hi).contains(&code) && dst.first() == Some(&b'<') {
                    let (v, n) = hex_val(dst);
                    return Some((v + (code - lo), n));
                }
            }
            _ => {}
        }
    }
    None
}

/// Accumula i byte di una stringa: senza CMap ogni byte è un carattere, con CMap ogni coppia è un glifo.
struct Text<'a> {
    cm: &'a [u8],
    w: u8, // byte per codice: 1 font semplice, 2 Type0 (Identity-H)
    pending: Option<u8>,
}

impl Text<'_> {
    fn feed(&mut self, b: u8, out: &mut [u8], o: &mut usize) -> Option<()> {
        if self.cm.is_empty() {
            return push_char(out, o, b as u32);
        }
        if self.w == 1 {
            if let Some((v, n)) = cmap_lookup(self.cm, b as u32) {
                emit_units(v, n, out, o)?;
            }
            return Some(());
        }
        match self.pending.take() {
            None => self.pending = Some(b),
            Some(hi) => {
                let code = (hi as u32) << 8 | b as u32;
                if let Some((v, n)) = cmap_lookup(self.cm, code) {
                    emit_units(v, n, out, o)?;
                }
            }
        }
        Some(())
    }
}

/// Emette il valore UTF-16BE di `n` byte come caratteri.
fn emit_units(v: u32, n: usize, out: &mut [u8], o: &mut usize) -> Option<()> {
    if n <= 1 {
        return push_char(out, o, v);
    }
    for k in (0..n / 2).rev() {
        push_char(out, o, (v >> (16 * k)) & 0xFFFF)?;
    }
    Some(())
}

/// Legge la CMap ToUnicode del font `name` in `cbuf`. Ritorna (lunghezza, byte per codice); (0, 1) se assente.
fn load_cmap(pdf: &[u8], name: &[u8], cbuf: &mut [u8]) -> (usize, u8) {
    let mut key = [0u8; 64];
    if name.len() + 2 > key.len() {
        return (0, 1);
    }
    key[0] = b'/';
    key[1..1 + name.len()].copy_from_slice(name);
    key[1 + name.len()] = b' ';
    let key = &key[..2 + name.len()];
    let mut from = 0;
    while let Some(p) = find(pdf, key, from) {
        from = p + 1;
        let Some(fnum) = parse_ref(pdf, p + key.len()) else { continue };
        let Some(fbody) = obj_body(pdf, fnum) else { continue };
        let Some(tp) = find(fbody, b"/ToUnicode", 0) else { continue };
        let Some(tnum) = parse_ref(fbody, tp + 10) else { continue };
        let Some(tbody) = obj_body(pdf, tnum) else { continue };
        let Some(s) = find(tbody, b"stream", 0) else { continue };
        let Some((dict, data)) = stream_at(tbody, s) else { continue };
        let w = if find(fbody, b"/Type0", 0).is_some() { 2 } else { 1 };
        return (decode(dict, data, cbuf).unwrap_or(0), w);
    }
    (0, 1)
}

/// Testo di un flusso di contenuto: operatori Tj, TJ, Tf, Td, T*, ET.
fn content(pdf: &[u8], c: &[u8], cbuf: &mut [u8], out: &mut [u8], o: &mut usize) -> Option<()> {
    let mut i = 0;
    let mut last_name: &[u8] = b"";
    let mut clen = 0usize;
    let mut cw = 1u8;
    let mut stack_name: &[u8] = b"";
    while i < c.len() {
        let b = c[i];
        match b {
            b'%' => {
                while i < c.len() && c[i] != b'\n' && c[i] != b'\r' {
                    i += 1;
                }
            }
            b'(' => {
                i += 1;
                let mut depth = 1;
                let mut t = Text { cm: &cbuf[..clen], w: cw, pending: None };
                while depth > 0 {
                    let ch = *c.get(i)?;
                    i += 1;
                    match ch {
                        b'\\' => {
                            let e = *c.get(i)?;
                            i += 1;
                            let v = match e {
                                b'n' => b'\n',
                                b'r' => b'\r',
                                b't' => b'\t',
                                b'b' => 8,
                                b'f' => 12,
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
                                    v as u8
                                }
                                b'\r' | b'\n' => continue,
                                _ => e,
                            };
                            t.feed(v, out, o)?;
                        }
                        b'(' => {
                            depth += 1;
                            t.feed(ch, out, o)?;
                        }
                        b')' => {
                            depth -= 1;
                            if depth > 0 {
                                t.feed(ch, out, o)?;
                            }
                        }
                        _ => t.feed(ch, out, o)?,
                    }
                }
            }
            b'<' if c.get(i + 1) != Some(&b'<') => {
                i += 1;
                let mut t = Text { cm: &cbuf[..clen], w: cw, pending: None };
                let mut hi: Option<u8> = None;
                loop {
                    let ch = *c.get(i)?;
                    i += 1;
                    if ch == b'>' {
                        break;
                    }
                    let v = match ch {
                        b'0'..=b'9' => ch - b'0',
                        b'a'..=b'f' => ch - b'a' + 10,
                        b'A'..=b'F' => ch - b'A' + 10,
                        _ => continue,
                    };
                    match hi.take() {
                        None => hi = Some(v),
                        Some(h) => t.feed(h * 16 + v, out, o)?,
                    }
                }
            }
            b'<' | b'>' => i += if c.get(i + 1) == Some(&b) { 2 } else { 1 },
            b'/' => {
                let s = i + 1;
                i = s;
                while i < c.len() && !is_delim(c[i]) {
                    i += 1;
                }
                stack_name = &c[s..i];
                last_name = stack_name;
            }
            _ if b.is_ascii_whitespace() || matches!(b, b'[' | b']' | b'{' | b'}' | b')') => i += 1,
            _ => {
                let s = i;
                while i < c.len() && !is_delim(c[i]) {
                    i += 1;
                }
                if i == s {
                    i += 1;
                    continue;
                }
                let tok = &c[s..i];
                if tok[0].is_ascii_digit() || matches!(tok[0], b'-' | b'.' | b'+') {
                    // Numero molto negativo in TJ: spazio tra parole.
                    if tok[0] == b'-' && tok.len() >= 4 && tok[1..].iter().all(|d| d.is_ascii_digit()) {
                        push(out, o, b' ')?;
                    }
                } else if tok == b"Tf" {
                    (clen, cw) = load_cmap(pdf, last_name, cbuf);
                } else if matches!(tok, b"Td" | b"TD" | b"T*" | b"Tm" | b"ET" | b"'" | b"\"") {
                    newline(out, o)?;
                }
            }
        }
    }
    Some(())
}

/// Testo del PDF. `scratch` tiene i flussi di contenuto, `cbuf` le CMap dei font.
pub fn read_text(pdf: &[u8], scratch: &mut [u8], cbuf: &mut [u8], out: &mut [u8]) -> Option<usize> {
    let mut o = 0;
    let mut from = 0;
    while let Some(s) = find(pdf, b"stream", from) {
        from = s + 6;
        if s >= 3 && &pdf[s - 3..s] == b"end" {
            continue;
        }
        let Some((dict, data)) = stream_at(pdf, s) else { continue };
        let Some(n) = decode(dict, data, scratch) else { continue };
        let body = &scratch[..n];
        // Salta font, CMap e metadati: non sono testo di pagina.
        if find(dict, b"/Length1", 0).is_some() || find(body, b"begincmap", 0).is_some() {
            continue;
        }
        content(pdf, body, cbuf, out, &mut o)?;
    }
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn testo_di_pdf_semplice() {
        let mut scratch = [0u8; 4096];
        let mut cbuf = [0u8; 4096];
        let mut out = [0u8; 512];
        let n = read_text(include_bytes!("../tests/fixtures/simple.pdf"), &mut scratch, &mut cbuf, &mut out).unwrap();
        assert_eq!(&out[..n], "Haiku office PDF\nRiga  due\nCiao (mondo)\n".as_bytes());
    }

    #[test]
    fn identity_h_con_tounicode() {
        let mut scratch = [0u8; 8192];
        let mut cbuf = [0u8; 8192];
        let mut out = [0u8; 512];
        let n = read_text(include_bytes!("../tests/fixtures/sample.pdf"), &mut scratch, &mut cbuf, &mut out).unwrap();
        assert_eq!(&out[..n], "Haiku office prova\nseconda riga\n".as_bytes());
    }
}
