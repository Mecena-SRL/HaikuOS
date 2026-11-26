//! Tokenizer XML minimo, senza allocazione (no_std). Non valida il documento.
//! Ignora commenti, dichiarazioni (`<?…?>`, `<!…>`). Niente CDATA: il testo resta grezzo
//! (le entità vanno decodificate dal chiamante con `unescape`).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tok<'a> {
    /// Contenuto del tag senza `<` e `>`, es. `w:t xml:space="preserve"` o `text:s/`.
    Start(&'a [u8]),
    /// Nome del tag di chiusura, es. `w:p`.
    End(&'a [u8]),
    /// Testo grezzo tra due tag.
    Text(&'a [u8]),
}

pub struct Tokens<'a> {
    s: &'a [u8],
    i: usize,
}

pub fn tokens(s: &[u8]) -> Tokens<'_> {
    Tokens { s, i: 0 }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Tok<'a>;

    fn next(&mut self) -> Option<Tok<'a>> {
        let s = self.s;
        while self.i < s.len() {
            if s[self.i] != b'<' {
                let st = self.i;
                while self.i < s.len() && s[self.i] != b'<' {
                    self.i += 1;
                }
                return Some(Tok::Text(&s[st..self.i]));
            }
            let rest = &s[self.i..];
            if rest.starts_with(b"<!--") {
                self.i += find(rest, b"-->").map_or(rest.len(), |p| p + 3);
                continue;
            }
            let close = find(rest, b">").unwrap_or(rest.len() - 1);
            let inner = &rest[1..close];
            self.i += close + 1;
            match inner.first() {
                Some(b'?') | Some(b'!') => continue,
                Some(b'/') => return Some(Tok::End(&inner[1..])),
                _ => return Some(Tok::Start(inner)),
            }
        }
        None
    }
}

fn find(h: &[u8], n: &[u8]) -> Option<usize> {
    h.windows(n.len()).position(|w| w == n)
}

/// Nome del tag (senza attributi, senza `/` finale).
pub fn name(tag: &[u8]) -> &[u8] {
    let end = tag
        .iter()
        .position(|&c| c == b' ' || c == b'/' || c == b'\n' || c == b'\t' || c == b'\r')
        .unwrap_or(tag.len());
    &tag[..end]
}

/// Tag auto-chiuso (`<x … />`).
pub fn is_empty(tag: &[u8]) -> bool {
    tag.last() == Some(&b'/')
}

/// Valore di un attributo `key="…"` (o con apici singoli).
pub fn attr<'a>(tag: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut i = 0;
    while i < tag.len() {
        if tag[i..].starts_with(key) && tag.get(i + key.len()) == Some(&b'=') {
            let q = *tag.get(i + key.len() + 1)?;
            let st = i + key.len() + 2;
            let len = tag[st..].iter().position(|&c| c == q)?;
            return Some(&tag[st..st + len]);
        }
        i += 1;
    }
    None
}

/// Numero decimale, 0 se assente o non valido.
pub fn num(v: Option<&[u8]>) -> usize {
    v.map_or(0, |v| {
        v.iter()
            .filter(|c| c.is_ascii_digit())
            .fold(0usize, |a, &c| a.saturating_mul(10).saturating_add((c - b'0') as usize))
    })
}
