//! Fogli: griglia fissa di celle di testo. Formato nativo CSV (RFC 4180), e xlsx minimo.
//! Le formule non sono ancora calcolate: **rinviato** (tappa 2b). Solo `core`, nessuna allocazione.

use crate::formats::{push, tag, text};
use crate::zip;

pub const CELL_MAX: usize = 24;

#[derive(Debug, PartialEq, Eq)]
pub enum SheetError {
    /// Riga o colonna oltre la griglia.
    Full,
    /// Testo di una cella più lungo di `CELL_MAX`.
    TooLong,
    /// CSV o xlsx non valido.
    Bad,
}

#[derive(Clone, Copy)]
pub struct Cell {
    n: u8,
    t: [u8; CELL_MAX],
}

impl Cell {
    const EMPTY: Cell = Cell { n: 0, t: [0; CELL_MAX] };

    pub fn text(&self) -> &[u8] {
        &self.t[..self.n as usize]
    }

    fn push(&mut self, b: u8) -> Result<(), SheetError> {
        if self.n as usize == CELL_MAX {
            return Err(SheetError::TooLong);
        }
        self.t[self.n as usize] = b;
        self.n += 1;
        Ok(())
    }

    fn set(&mut self, s: &[u8]) -> Result<(), SheetError> {
        self.n = 0;
        for &b in s {
            self.push(b)?;
        }
        Ok(())
    }
}

pub struct Sheet<const R: usize, const C: usize> {
    cells: [[Cell; C]; R],
}

impl<const R: usize, const C: usize> Sheet<R, C> {
    pub const fn new() -> Self {
        Sheet { cells: [[Cell::EMPTY; C]; R] }
    }

    pub fn get(&self, r: usize, c: usize) -> &[u8] {
        self.cells.get(r).and_then(|row| row.get(c)).map(|x| x.text()).unwrap_or(b"")
    }

    pub fn set(&mut self, r: usize, c: usize, s: &[u8]) -> Result<(), SheetError> {
        self.cells.get_mut(r).and_then(|row| row.get_mut(c)).ok_or(SheetError::Full)?.set(s)
    }

    fn cell_mut(&mut self, r: usize, c: usize) -> Result<&mut Cell, SheetError> {
        self.cells.get_mut(r).and_then(|row| row.get_mut(c)).ok_or(SheetError::Full)
    }

    /// Ultima riga e colonna non vuote, +1 (0 se il foglio è vuoto).
    fn used(&self) -> (usize, usize) {
        let (mut rows, mut cols) = (0, 0);
        for (r, row) in self.cells.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                if cell.n > 0 {
                    rows = rows.max(r + 1);
                    cols = cols.max(c + 1);
                }
            }
        }
        (rows, cols)
    }

    /// Carica un CSV. Il foglio viene svuotato prima. Ogni riga è LF o CRLF.
    pub fn load_csv(&mut self, src: &[u8]) -> Result<(), SheetError> {
        *self = Sheet::new();
        let (mut r, mut c, mut i) = (0usize, 0usize, 0usize);
        let mut quoted = false;
        let mut field_started = false;
        while i < src.len() {
            let b = src[i];
            i += 1;
            if quoted {
                if b == b'"' {
                    if src.get(i) == Some(&b'"') {
                        i += 1;
                        self.cell_mut(r, c)?.push(b'"')?;
                    } else {
                        quoted = false;
                    }
                } else {
                    self.cell_mut(r, c)?.push(b)?;
                }
                continue;
            }
            match b {
                b'"' if !field_started => {
                    quoted = true;
                    field_started = true;
                }
                b',' => {
                    c += 1;
                    field_started = false;
                    if c >= C {
                        return Err(SheetError::Full);
                    }
                }
                b'\n' => {
                    r += 1;
                    c = 0;
                    field_started = false;
                    if r >= R {
                        return Err(SheetError::Full);
                    }
                }
                b'\r' => {}
                _ => {
                    field_started = true;
                    self.cell_mut(r, c)?.push(b)?;
                }
            }
        }
        if quoted {
            return Err(SheetError::Bad);
        }
        Ok(())
    }

    /// Scrive il CSV. Ritorna i byte scritti.
    pub fn write_csv(&self, out: &mut [u8]) -> Option<usize> {
        let (rows, cols) = self.used();
        let mut o = 0;
        for r in 0..rows {
            for c in 0..cols {
                if c > 0 {
                    push(out, &mut o, b',')?;
                }
                let s = self.get(r, c);
                let quote = s.iter().any(|&b| matches!(b, b',' | b'"' | b'\n'));
                if quote {
                    push(out, &mut o, b'"')?;
                }
                for &b in s {
                    if b == b'"' {
                        push(out, &mut o, b'"')?;
                    }
                    push(out, &mut o, b)?;
                }
                if quote {
                    push(out, &mut o, b'"')?;
                }
            }
            push(out, &mut o, b'\n')?;
        }
        Some(o)
    }

    /// Scrive un xlsx minimo (una sola scheda, testo inline). `xml` è un buffer di lavoro.
    pub fn write_xlsx(&self, xml: &mut [u8], out: &mut [u8]) -> Option<usize> {
        let (rows, cols) = self.used();
        let mut o = 0;
        for &b in b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>" {
            push(xml, &mut o, b)?;
        }
        for r in 0..rows {
            let mut rowtag = [0u8; 16];
            let rn = fmt_num(r + 1, &mut rowtag);
            for &b in b"<row r=\"" { push(xml, &mut o, b)?; }
            for &b in rn { push(xml, &mut o, b)?; }
            push(xml, &mut o, b'"')?;
            push(xml, &mut o, b'>')?;
            for c in 0..cols {
                let s = self.get(r, c);
                if s.is_empty() {
                    continue;
                }
                for &b in b"<c r=\"" { push(xml, &mut o, b)?; }
                if c >= 26 {
                    return None;
                }
                push(xml, &mut o, b'A' + c as u8)?;
                for &b in rn { push(xml, &mut o, b)?; }
                for &b in b"\" t=\"inlineStr\"><is><t xml:space=\"preserve\">" { push(xml, &mut o, b)?; }
                for &b in s {
                    let esc: &[u8] = match b {
                        b'&' => b"&amp;",
                        b'<' => b"&lt;",
                        b'>' => b"&gt;",
                        _ => &[b],
                    };
                    for &e in esc { push(xml, &mut o, e)?; }
                }
                for &b in b"</t></is></c>" { push(xml, &mut o, b)?; }
            }
            for &b in b"</row>" { push(xml, &mut o, b)?; }
        }
        for &b in b"</sheetData></worksheet>" { push(xml, &mut o, b)?; }
        zip::write(
            &[
                (b"[Content_Types].xml", CT_XLSX),
                (b"_rels/.rels", RELS_XLSX),
                (b"xl/workbook.xml", WORKBOOK),
                (b"xl/_rels/workbook.xml.rels", WB_RELS),
                (b"xl/worksheets/sheet1.xml", &xml[..o]),
            ],
            out,
        )
    }

    /// Legge la prima scheda di un xlsx (testo inline o condiviso, valori numerici come testo).
    /// `xml` e `sst` sono buffer di lavoro.
    pub fn read_xlsx(&mut self, xlsx: &[u8], xml: &mut [u8], sst: &mut [u8]) -> Result<(), SheetError> {
        *self = Sheet::new();
        let n = zip::extract(xlsx, b"xl/worksheets/sheet1.xml", xml).ok_or(SheetError::Bad)?;
        let sn = zip::extract(xlsx, b"xl/sharedStrings.xml", sst).unwrap_or(0);
        let x = &xml[..n];
        let mut i = 0;
        while let Some(p) = x[i..].iter().position(|&b| b == b'<') {
            let s = i + p;
            let (name, close, selfclose, next) = tag(x, s).ok_or(SheetError::Bad)?;
            i = next;
            if name != b"c" || close || selfclose {
                continue;
            }
            let inner = &x[s + 1..next - 1];
            let r = attr(inner, b"r").ok_or(SheetError::Bad)?;
            let (row, col) = addr(r)?;
            let kind = attr(inner, b"t");
            let end = i + find(&x[i..], b"</c>").ok_or(SheetError::Bad)?;
            let body = &x[i..end];
            i = end + 4;
            let value: &[u8] = if kind == Some(&b"s"[..]) {
                let k = num(body_between(body, b"<v>", b"</v>").ok_or(SheetError::Bad)?);
                shared(&sst[..sn], k).ok_or(SheetError::Bad)?
            } else if kind == Some(&b"inlineStr"[..]) {
                body_between(body, b"<t", b"</t>").map(after_gt).unwrap_or(b"")
            } else {
                body_between(body, b"<v>", b"</v>").unwrap_or(b"")
            };
            let mut tmp = [0u8; CELL_MAX * 2];
            let mut o = 0;
            text(value, &mut tmp, &mut o).ok_or(SheetError::TooLong)?;
            self.cell_mut(row, col)?.set(&tmp[..o])?;
        }
        Ok(())
    }
}

const CT_XLSX: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/></Types>";
const RELS_XLSX: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>";
const WORKBOOK: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets><sheet name=\"Foglio1\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>";
const WB_RELS: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/></Relationships>";

fn fmt_num(mut v: usize, buf: &mut [u8; 16]) -> &[u8] {
    let mut k = buf.len();
    loop {
        k -= 1;
        buf[k] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    &buf[k..]
}

fn num(s: &[u8]) -> usize {
    s.iter().filter(|b| b.is_ascii_digit()).fold(0, |a, &d| a * 10 + (d - b'0') as usize)
}

/// Testo dopo il primo `>` (per gli elementi `<t ...>testo`).
fn after_gt(s: &[u8]) -> &[u8] {
    match s.iter().position(|&b| b == b'>') {
        Some(p) => &s[p + 1..],
        None => b"",
    }
}

fn find(h: &[u8], pat: &[u8]) -> Option<usize> {
    h.windows(pat.len()).position(|w| w == pat)
}

/// Valore di un attributo `name="..."` dentro un tag (senza `<` e `>`).
fn attr<'a>(inner: &'a [u8], name: &[u8]) -> Option<&'a [u8]> {
    let mut pat = [0u8; 8];
    pat[..name.len()].copy_from_slice(name);
    pat[name.len()..name.len() + 2].copy_from_slice(b"=\"");
    let p = find(inner, &pat[..name.len() + 2])? + name.len() + 2;
    let e = inner[p..].iter().position(|&b| b == b'"')?;
    Some(&inner[p..p + e])
}

/// "B3" -> (riga 2, colonna 1), indici da zero.
fn addr(r: &[u8]) -> Result<(usize, usize), SheetError> {
    let split = r.iter().position(|b| b.is_ascii_digit()).ok_or(SheetError::Bad)?;
    let mut col = 0usize;
    for &b in &r[..split] {
        if !b.is_ascii_uppercase() {
            return Err(SheetError::Bad);
        }
        col = col * 26 + (b - b'A' + 1) as usize;
    }
    let row = num(&r[split..]);
    if col == 0 || row == 0 {
        return Err(SheetError::Bad);
    }
    Ok((row - 1, col - 1))
}

fn body_between<'a>(h: &'a [u8], open: &[u8], close: &[u8]) -> Option<&'a [u8]> {
    let s = find(h, open)? + open.len();
    let e = s + find(&h[s..], close)?;
    Some(&h[s..e])
}

/// k-esima stringa condivisa (<si>), testo del primo <t>.
fn shared(sst: &[u8], k: usize) -> Option<&[u8]> {
    let mut from = 0;
    for _ in 0..=k {
        from += find(&sst[from..], b"<si>")? + 4;
    }
    let body = &sst[from..];
    let end = find(body, b"</si>").unwrap_or(body.len());
    body_between(&body[..end], b"<t", b"</t>").map(after_gt)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_con_virgolette() {
        let mut s: Sheet<8, 4> = Sheet::new();
        s.load_csv(b"a,\"b,c\"\r\n\"dici \"\"ciao\"\"\",3\n").unwrap();
        assert_eq!(s.get(0, 1), b"b,c");
        assert_eq!(s.get(1, 0), b"dici \"ciao\"");
        let mut out = [0u8; 128];
        let n = s.write_csv(&mut out).unwrap();
        assert_eq!(&out[..n], b"a,\"b,c\"\n\"dici \"\"ciao\"\"\",3\n");
    }

    #[test]
    fn fuori_griglia_e_troppo_lungo() {
        let mut s: Sheet<2, 2> = Sheet::new();
        assert_eq!(s.load_csv(b"1,2,3\n"), Err(SheetError::Full));
        assert_eq!(s.set(0, 0, &[b'x'; CELL_MAX + 1]), Err(SheetError::TooLong));
    }

    #[test]
    fn xlsx_andata_e_ritorno() {
        let mut s: Sheet<4, 3> = Sheet::new();
        s.set(0, 0, b"Titolo & <x>").unwrap();
        s.set(2, 1, b"42").unwrap();
        let mut xml = [0u8; 2048];
        let mut z = [0u8; 4096];
        let n = s.write_xlsx(&mut xml, &mut z).unwrap();
        let mut t: Sheet<4, 3> = Sheet::new();
        let (mut x2, mut sst) = ([0u8; 2048], [0u8; 64]);
        t.read_xlsx(&z[..n], &mut x2, &mut sst).unwrap();
        assert_eq!(t.get(0, 0), b"Titolo & <x>");
        assert_eq!(t.get(2, 1), b"42");
        assert_eq!(t.get(1, 1), b"");
    }

    #[test]
    fn xlsx_di_libreoffice() {
        let mut s: Sheet<8, 4> = Sheet::new();
        let (mut x, mut sst) = ([0u8; 4096], [0u8; 4096]);
        s.read_xlsx(include_bytes!("../tests/fixtures/sample.xlsx"), &mut x, &mut sst).unwrap();
        assert_eq!(s.get(0, 0), b"nome");
        assert_eq!(s.get(1, 0), b"Mela, rossa");
        assert_eq!(s.get(1, 1), b"3");
        assert_eq!(s.get(2, 0), b"Pera");
    }
}

/// Parser delle formule: `+ - * /`, parentesi, numeri, riferimenti (`A1`), `SUM(A1:B3)`.
struct Parser<'a, const R: usize, const C: usize> {
    s: &'a [u8],
    i: usize,
    sh: &'a Sheet<R, C>,
    depth: usize,
}

impl<const R: usize, const C: usize> Parser<'_, R, C> {
    fn ws(&mut self) {
        while self.s.get(self.i) == Some(&b' ') {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.i).copied()
    }

    fn expr(&mut self) -> Option<f64> {
        let mut v = self.term()?;
        while let Some(op @ (b'+' | b'-')) = self.peek() {
            self.i += 1;
            let r = self.term()?;
            v = if op == b'+' { v + r } else { v - r };
        }
        Some(v)
    }

    fn term(&mut self) -> Option<f64> {
        let mut v = self.factor()?;
        while let Some(op @ (b'*' | b'/')) = self.peek() {
            self.i += 1;
            let r = self.factor()?;
            if op == b'*' {
                v *= r;
            } else {
                if r == 0.0 {
                    return None;
                }
                v /= r;
            }
        }
        Some(v)
    }

    fn factor(&mut self) -> Option<f64> {
        match self.peek()? {
            b'-' => {
                self.i += 1;
                Some(-self.factor()?)
            }
            b'(' => {
                self.i += 1;
                let v = self.expr()?;
                (self.peek()? == b')').then_some(())?;
                self.i += 1;
                Some(v)
            }
            b'0'..=b'9' => self.number(),
            b'A'..=b'Z' => self.name(),
            _ => None,
        }
    }

    fn number(&mut self) -> Option<f64> {
        let mut v = 0.0;
        while let Some(&d @ b'0'..=b'9') = self.s.get(self.i) {
            v = v * 10.0 + (d - b'0') as f64;
            self.i += 1;
        }
        if self.s.get(self.i) == Some(&b'.') {
            self.i += 1;
            let mut f = 0.1;
            while let Some(&d @ b'0'..=b'9') = self.s.get(self.i) {
                v += (d - b'0') as f64 * f;
                f /= 10.0;
                self.i += 1;
            }
        }
        Some(v)
    }

    /// Riferimento di cella o funzione SUM(intervallo).
    fn name(&mut self) -> Option<f64> {
        let s = self.i;
        while self.s.get(self.i).is_some_and(|b| b.is_ascii_uppercase() || b.is_ascii_digit()) {
            self.i += 1;
        }
        let word = &self.s[s..self.i];
        if self.s.get(self.i) == Some(&b'(') {
            if word != b"SUM" {
                return None;
            }
            self.i += 1;
            let a = self.cell_ref()?;
            (self.s.get(self.i) == Some(&b':')).then_some(())?;
            self.i += 1;
            let b = self.cell_ref()?;
            (self.s.get(self.i) == Some(&b')')).then_some(())?;
            self.i += 1;
            let mut sum = 0.0;
            for r in a.0.min(b.0)..=a.0.max(b.0) {
                for c in a.1.min(b.1)..=a.1.max(b.1) {
                    if let Some(v) = self.sh.value_at(r, c, self.depth + 1) {
                        sum += v;
                    }
                }
            }
            return Some(sum);
        }
        self.i = s;
        let (r, c) = self.cell_ref()?;
        self.sh.value_at(r, c, self.depth + 1)
    }

    fn cell_ref(&mut self) -> Option<(usize, usize)> {
        self.ws();
        let s = self.i;
        while self.s.get(self.i).is_some_and(|b| b.is_ascii_uppercase() || b.is_ascii_digit()) {
            self.i += 1;
        }
        addr(&self.s[s..self.i]).ok()
    }
}

impl<const R: usize, const C: usize> Sheet<R, C> {
    /// Valore numerico di una cella: numero, oppure formula che inizia con `=`.
    pub fn value(&self, r: usize, c: usize) -> Option<f64> {
        self.value_at(r, c, 0)
    }

    fn value_at(&self, r: usize, c: usize, depth: usize) -> Option<f64> {
        if depth > 16 {
            return None; // riferimenti circolari
        }
        let t = self.get(r, c);
        if t.first() == Some(&b'=') {
            let mut p = Parser { s: &t[1..], i: 0, sh: self, depth };
            let v = p.expr()?;
            p.ws();
            return (p.i == p.s.len()).then_some(v);
        }
        let mut p = Parser { s: t, i: 0, sh: self, depth };
        let v = p.number()?;
        (p.i == t.len() && !t.is_empty()).then_some(v)
    }
}

#[cfg(test)]
mod formule {
    use super::*;

    #[test]
    fn formule_base() {
        let mut s: Sheet<6, 4> = Sheet::new();
        s.set(0, 0, b"2").unwrap();
        s.set(1, 0, b"3").unwrap();
        s.set(2, 0, b"=A1+A2*2").unwrap();
        s.set(3, 0, b"=SUM(A1:A3)").unwrap();
        s.set(4, 0, b"=(A1-1)/0").unwrap();
        s.set(5, 0, b"=A6").unwrap();
        assert_eq!(s.value(2, 0), Some(8.0));
        assert_eq!(s.value(3, 0), Some(13.0));
        assert_eq!(s.value(4, 0), None);
        assert_eq!(s.value(5, 0), None);
    }
}
