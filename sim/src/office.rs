//! Estrazione del testo da DOCX, ODT, ODS e XLSX (sola lettura), senza allocazione nel nucleo.
//! Il nucleo scrive in un `Sink` a dimensione fissa. La funzione `read_text` (feature `std`)
//! è il wrapper per il simulatore: rileva il formato e restituisce una `String`.
//! Regole e limiti: docs/office-formats.md.

use crate::xml::{attr, is_empty, name, num, tokens, Tok};
use crate::zip::Error;

/// Buffer di uscita a dimensione fissa. Scrive testo UTF-8 e decodifica le entità XML.
pub struct Sink<'a> {
    buf: &'a mut [u8],
    n: usize,
}

impl<'a> Sink<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Sink { buf, n: 0 }
    }

    pub fn len(&self) -> usize {
        self.n
    }

    fn byte(&mut self, b: u8) -> Result<(), Error> {
        *self.buf.get_mut(self.n).ok_or(Error::Full)? = b;
        self.n += 1;
        Ok(())
    }

    fn bytes(&mut self, b: &[u8]) -> Result<(), Error> {
        for &c in b {
            self.byte(c)?;
        }
        Ok(())
    }

    /// Fine riga: toglie le tabulazioni finali (celle vuote) e scrive `\n`.
    fn newline(&mut self) -> Result<(), Error> {
        while self.n > 0 && self.buf[self.n - 1] == b'\t' {
            self.n -= 1;
        }
        self.byte(b'\n')
    }

    fn tabs(&mut self, k: usize) -> Result<(), Error> {
        for _ in 0..k.min(256) {
            self.byte(b'\t')?;
        }
        Ok(())
    }

    /// Testo grezzo XML con entità decodificate (`&amp;`, `&#233;`, `&#xE9;` …).
    fn text(&mut self, raw: &[u8]) -> Result<(), Error> {
        let mut i = 0;
        while i < raw.len() {
            if raw[i] != b'&' {
                self.byte(raw[i])?;
                i += 1;
                continue;
            }
            let end = raw[i..].iter().take(12).position(|&c| c == b';').map(|p| i + p);
            match end.and_then(|e| entity(&raw[i + 1..e]).map(|ch| (ch, e))) {
                Some((ch, e)) => {
                    let mut b = [0u8; 4];
                    self.bytes(ch.encode_utf8(&mut b).as_bytes())?;
                    i = e + 1;
                }
                None => {
                    self.byte(b'&')?;
                    i += 1;
                }
            }
        }
        Ok(())
    }
}

fn entity(n: &[u8]) -> Option<char> {
    match n {
        b"amp" => Some('&'),
        b"lt" => Some('<'),
        b"gt" => Some('>'),
        b"quot" => Some('"'),
        b"apos" => Some('\''),
        _ if n.len() > 1 && n[0] == b'#' => {
            let (radix, d) = if n[1] == b'x' { (16, &n[2..]) } else { (10, &n[1..]) };
            if d.is_empty() {
                return None;
            }
            let mut v: u32 = 0;
            for &c in d {
                v = v.checked_mul(radix)?.checked_add((c as char).to_digit(radix)?)?;
            }
            char::from_u32(v)
        }
        _ => None,
    }
}

fn blank(x: &[u8]) -> bool {
    x.iter().all(|c| c.is_ascii_whitespace())
}

/// DOCX, parte `word/document.xml`. Paragrafi a capo, `w:tab` e `w:br` a tabulazione e a capo.
pub fn docx(xml: &[u8], s: &mut Sink) -> Result<(), Error> {
    let mut in_t = false;
    for tok in tokens(xml) {
        match tok {
            Tok::Start(t) => match name(t) {
                b"w:t" if !is_empty(t) => in_t = true,
                b"w:tab" => s.byte(b'\t')?,
                b"w:br" | b"w:cr" => s.byte(b'\n')?,
                _ => {}
            },
            Tok::End(t) => match name(t) {
                b"w:t" => in_t = false,
                b"w:p" => s.byte(b'\n')?,
                _ => {}
            },
            Tok::Text(x) if in_t => s.text(x)?,
            Tok::Text(_) => {}
        }
    }
    Ok(())
}

/// ODT, parte `content.xml`. Paragrafi e titoli a capo, `text:s` a spazi, `text:tab` a tabulazione.
pub fn odt(xml: &[u8], s: &mut Sink) -> Result<(), Error> {
    for tok in tokens(xml) {
        match tok {
            Tok::Start(t) => match name(t) {
                b"text:s" => {
                    let c = if attr(t, b"text:c").is_some() { num(attr(t, b"text:c")).max(1) } else { 1 };
                    for _ in 0..c.min(256) {
                        s.byte(b' ')?;
                    }
                }
                b"text:tab" => s.byte(b'\t')?,
                b"text:line-break" => s.byte(b'\n')?,
                _ => {}
            },
            Tok::End(t) => match name(t) {
                b"text:p" | b"text:h" => s.byte(b'\n')?,
                _ => {}
            },
            Tok::Text(x) if !blank(x) => s.text(x)?,
            Tok::Text(_) => {}
        }
    }
    Ok(())
}

/// ODS, parte `content.xml`. Ogni foglio inizia col suo nome. Righe a capo, celle a tabulazione.
/// Celle ripetute (`table:number-columns-repeated`): tabulazioni multiple, testo scritto una volta.
pub fn ods(xml: &[u8], s: &mut Sink) -> Result<(), Error> {
    let mut cell_rep = 1usize;
    for tok in tokens(xml) {
        match tok {
            Tok::Start(t) => match name(t) {
                b"table:table" => {
                    if let Some(n) = attr(t, b"table:name") {
                        s.text(n)?;
                        s.byte(b'\n')?;
                    }
                }
                b"table:table-cell" | b"table:covered-table-cell" => {
                    cell_rep = num(attr(t, b"table:number-columns-repeated")).max(1);
                    if is_empty(t) {
                        s.tabs(cell_rep)?;
                    }
                }
                b"table:table-row" if is_empty(t) => s.newline()?,
                _ => {}
            },
            Tok::End(t) => match name(t) {
                b"table:table-row" => s.newline()?,
                b"table:table-cell" | b"table:covered-table-cell" => s.tabs(cell_rep)?,
                _ => {}
            },
            Tok::Text(x) if !blank(x) => s.text(x)?,
            Tok::Text(_) => {}
        }
    }
    Ok(())
}

/// Stringa n. `idx` della tabella condivisa (`xl/sharedStrings.xml`), scritta in `s`.
fn shared_string(sst: &[u8], idx: usize, s: &mut Sink) -> Result<(), Error> {
    let mut counter = 0usize;
    let (mut active, mut in_t, mut skip) = (false, false, false);
    for tok in tokens(sst) {
        match tok {
            Tok::Start(t) => match name(t) {
                b"si" => active = counter == idx,
                b"rPh" => skip = true,
                b"t" if active && !skip && !is_empty(t) => in_t = true,
                _ => {}
            },
            Tok::End(t) => match name(t) {
                b"si" => {
                    if active {
                        return Ok(());
                    }
                    counter += 1;
                }
                b"rPh" => skip = false,
                b"t" => in_t = false,
                _ => {}
            },
            Tok::Text(x) if in_t => s.text(x)?,
            Tok::Text(_) => {}
        }
    }
    Ok(())
}

/// Foglio XLSX (es. `xl/worksheets/sheet1.xml`). Righe a capo, celle a tabulazione.
/// `sst` è la tabella delle stringhe condivise, se presente. Ogni ricerca scandisce la tabella:
/// lento per fogli grandi, da migliorare con un indice (vedi docs/office-formats.md).
pub fn xlsx(sheet: &[u8], sst: Option<&[u8]>, s: &mut Sink) -> Result<(), Error> {
    let (mut shared, mut in_v, mut in_t) = (false, false, false);
    for tok in tokens(sheet) {
        match tok {
            Tok::Start(t) => match name(t) {
                b"c" => {
                    shared = attr(t, b"t") == Some(b"s");
                    if is_empty(t) {
                        s.byte(b'\t')?;
                    }
                }
                b"v" if !is_empty(t) => in_v = true,
                b"t" if !is_empty(t) => in_t = true,
                b"row" if is_empty(t) => s.newline()?,
                _ => {}
            },
            Tok::End(t) => match name(t) {
                b"c" => s.byte(b'\t')?,
                b"v" => in_v = false,
                b"t" => in_t = false,
                b"row" => s.newline()?,
                _ => {}
            },
            Tok::Text(x) if in_v && shared => {
                if let Some(sst) = sst {
                    shared_string(sst, num(Some(x)), s)?;
                }
            }
            Tok::Text(x) if in_v || in_t => s.text(x)?,
            Tok::Text(_) => {}
        }
    }
    Ok(())
}

#[cfg(feature = "std")]
mod host {
    use super::*;
    use crate::zip::Archive;

    /// Contenuto di una voce, intera.
    fn part(ar: &Archive, name: &str) -> Result<Vec<u8>, Error> {
        let e = ar.find(name)?;
        let mut buf = vec![0u8; e.usize_ as usize];
        let n = ar.extract(&e, &mut buf)?;
        buf.truncate(n);
        Ok(buf)
    }

    /// Esegue `f` con un buffer che cresce finché la sorgente sta dentro.
    fn render(f: impl Fn(&mut Sink) -> Result<(), Error>) -> Result<String, Error> {
        let mut cap = 4096usize;
        loop {
            let mut buf = vec![0u8; cap];
            let mut s = Sink::new(&mut buf);
            let r = f(&mut s);
            let n = s.len();
            match r {
                Ok(()) => return String::from_utf8(buf[..n].to_vec()).map_err(|_| Error::BadData),
                Err(Error::Full) if cap < (1 << 26) => cap *= 2,
                Err(e) => return Err(e),
            }
        }
    }

    /// Testo di un documento Office o OpenDocument. Il formato è riconosciuto dal contenuto.
    pub fn read_text(file: &[u8]) -> Result<String, Error> {
        let ar = Archive::open(file)?;
        if ar.find("word/document.xml").is_ok() {
            let xml = part(&ar, "word/document.xml")?;
            return render(|s| docx(&xml, s));
        }
        if ar.find("xl/workbook.xml").is_ok() {
            let sheet = part(&ar, "xl/worksheets/sheet1.xml")?;
            let sst = part(&ar, "xl/sharedStrings.xml").ok();
            return render(|s| xlsx(&sheet, sst.as_deref(), s));
        }
        let mime = part(&ar, "mimetype")?;
        let content = part(&ar, "content.xml")?;
        if mime.windows(24).any(|w| w == b"opendocument.spreadsheet") {
            return render(|s| ods(&content, s));
        }
        if mime.windows(17).any(|w| w == b"opendocument.text") {
            return render(|s| odt(&content, s));
        }
        Err(Error::Unsupported)
    }
}

#[cfg(feature = "std")]
pub use host::read_text;
