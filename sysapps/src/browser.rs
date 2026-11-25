//! Browser di testo: pagine in formato a righe, cronologia a capacità fissa.
//!
//! Formato pagina:
//! - `# Titolo`            intestazione
//! - `=> nome etichetta`   collegamento (l'etichetta è opzionale)
//! - altro                 testo

use crate::Store;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowseError {
    NotFound,
    /// URL di rete: il net stack non è ancora collegato.
    Offline,
    NoLink,
    NoPage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line<'a> {
    Heading(&'a str),
    Link { target: &'a str, label: &'a str },
    Text(&'a str),
}

pub fn parse_line(line: &str) -> Line<'_> {
    if let Some(title) = line.strip_prefix("# ") {
        return Line::Heading(title.trim());
    }
    if let Some(rest) = line.strip_prefix("=> ") {
        let rest = rest.trim();
        return match rest.find(' ') {
            Some(i) => Line::Link { target: &rest[..i], label: rest[i + 1..].trim() },
            None => Line::Link { target: rest, label: rest },
        };
    }
    Line::Text(line)
}

/// Indice `index` tra i collegamenti della pagina.
pub fn link_target(body: &'static str, index: usize) -> Option<&'static str> {
    body.lines()
        .filter_map(|l| match parse_line(l) {
            Line::Link { target, .. } => Some(target),
            _ => None,
        })
        .nth(index)
}

/// Cronologia a N voci. Visitare una pagina cancella il ramo "avanti".
pub struct History<const N: usize> {
    items: [&'static str; N],
    len: usize,
    pos: usize,
}

impl<const N: usize> History<N> {
    pub const fn new() -> Self {
        Self { items: [""; N], len: 0, pos: 0 }
    }

    pub fn visit(&mut self, body: &'static str) {
        if self.len > 0 {
            self.len = self.pos + 1;
        }
        if self.len == N {
            // capacità piena: scarta la voce più vecchia
            self.items.copy_within(1..N, 0);
            self.len = N - 1;
        }
        self.items[self.len] = body;
        self.len += 1;
        self.pos = self.len - 1;
    }

    pub fn back(&mut self) -> Option<&'static str> {
        if self.pos > 0 {
            self.pos -= 1;
            Some(self.items[self.pos])
        } else {
            None
        }
    }

    pub fn forward(&mut self) -> Option<&'static str> {
        if self.pos + 1 < self.len {
            self.pos += 1;
            Some(self.items[self.pos])
        } else {
            None
        }
    }

    pub fn current(&self) -> Option<&'static str> {
        (self.len > 0).then(|| self.items[self.pos])
    }
}

pub struct Browser<const N: usize> {
    history: History<N>,
}

impl<const N: usize> Browser<N> {
    pub const fn new() -> Self {
        Self { history: History::new() }
    }

    /// Apre un file dello store e lo mette in cronologia.
    pub fn open(&mut self, store: &dyn Store, target: &str) -> Result<&'static str, BrowseError> {
        let body = fetch(store, target)?;
        self.history.visit(body);
        Ok(body)
    }

    /// Segue il collegamento `index` della pagina corrente.
    pub fn follow(&mut self, store: &dyn Store, index: usize) -> Result<&'static str, BrowseError> {
        let body = self.history.current().ok_or(BrowseError::NoPage)?;
        let target = link_target(body, index).ok_or(BrowseError::NoLink)?;
        self.open(store, target)
    }

    pub fn back(&mut self) -> Option<&'static str> {
        self.history.back()
    }

    pub fn forward(&mut self) -> Option<&'static str> {
        self.history.forward()
    }

    pub fn current(&self) -> Option<&'static str> {
        self.history.current()
    }
}

fn fetch(store: &dyn Store, target: &str) -> Result<&'static str, BrowseError> {
    if target.starts_with("http://") || target.starts_with("https://") {
        return Err(BrowseError::Offline);
    }
    store.file(target).ok_or(BrowseError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Files;

    static STORE: Files = Files(&[
        ("home", "# Benvenuto\n=> pagina1 Prima pagina\n=> http://x.org\nTesto"),
        ("pagina1", "# Uno\n=> home\n=> pagina2"),
        ("pagina2", "# Due"),
    ]);

    #[test]
    fn parse_lines() {
        assert_eq!(parse_line("# T"), Line::Heading("T"));
        assert_eq!(parse_line("=> a Lab el"), Line::Link { target: "a", label: "Lab el" });
        assert_eq!(parse_line("=> a"), Line::Link { target: "a", label: "a" });
        assert_eq!(parse_line("ciao"), Line::Text("ciao"));
    }

    #[test]
    fn open_follow_back_forward() {
        let mut b: Browser<8> = Browser::new();
        assert_eq!(b.open(&STORE, "home").unwrap(), STORE.file("home").unwrap());
        b.follow(&STORE, 0).unwrap();
        assert_eq!(b.current(), STORE.file("pagina1"));
        assert_eq!(b.back(), STORE.file("home"));
        assert_eq!(b.back(), None);
        assert_eq!(b.forward(), STORE.file("pagina1"));
        assert_eq!(b.forward(), None);
    }

    #[test]
    fn visit_drops_forward_branch() {
        let mut b: Browser<8> = Browser::new();
        b.open(&STORE, "home").unwrap();
        b.open(&STORE, "pagina1").unwrap();
        b.back();
        b.open(&STORE, "pagina2").unwrap();
        assert_eq!(b.forward(), None);
        assert_eq!(b.back(), STORE.file("home"));
    }

    #[test]
    fn capacity_keeps_newest() {
        let mut b: Browser<2> = Browser::new();
        b.open(&STORE, "home").unwrap();
        b.open(&STORE, "pagina1").unwrap();
        b.open(&STORE, "pagina2").unwrap();
        assert_eq!(b.back(), STORE.file("pagina1"));
        assert_eq!(b.back(), None);
    }

    #[test]
    fn errors() {
        let mut b: Browser<4> = Browser::new();
        assert_eq!(b.open(&STORE, "nope"), Err(BrowseError::NotFound));
        assert_eq!(b.open(&STORE, "https://a.b"), Err(BrowseError::Offline));
        assert_eq!(b.follow(&STORE, 0), Err(BrowseError::NoPage));
        b.open(&STORE, "pagina2").unwrap();
        assert_eq!(b.follow(&STORE, 0), Err(BrowseError::NoLink));
    }
}
