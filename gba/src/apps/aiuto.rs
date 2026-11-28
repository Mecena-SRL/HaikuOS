//! Aiuto: argomenti di guida con navigazione su/giù. Testi brevi, solo ASCII per il font GBA.

pub const ARGOMENTI: [(&str, &[&str]); 4] = [
    ("MUOVERSI", &["SU E GIU", "SCEGLIE UNA VOCE", "B CHIUDE"]),
    ("FINESTRE", &["OGNI APP APRE", "UNA FINESTRA", "SULLA SCRIVANIA"]),
    ("MEMORIA", &["OGNI APP HA", "UNO SLOT.", "SE MANCA SPAZIO", "L'AVVIO E NEGATO"]),
    ("SPEGNERE", &["CHIUDI LE APP", "E TORNA AL MENU"]),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Aiuto {
    sel: u8,
}

impl Aiuto {
    pub const fn new() -> Self {
        Aiuto { sel: 0 }
    }

    pub fn sel(&self) -> usize {
        self.sel as usize
    }

    pub fn titolo(&self) -> &'static str {
        ARGOMENTI[self.sel()].0
    }

    pub fn righe(&self) -> &'static [&'static str] {
        ARGOMENTI[self.sel()].1
    }

    pub fn su(&mut self) {
        self.sel = self.sel.saturating_sub(1);
    }

    pub fn giu(&mut self) {
        if (self.sel as usize) + 1 < ARGOMENTI.len() {
            self.sel += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigazione_resta_nei_limiti() {
        let mut a = Aiuto::new();
        a.su();
        assert_eq!(a.titolo(), "MUOVERSI");
        for _ in 0..10 {
            a.giu();
        }
        assert_eq!(a.titolo(), "SPEGNERE");
    }
}
