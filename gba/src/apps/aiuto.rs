//! Aiuto: argomenti di guida con navigazione su/giù. Testi brevi, solo ASCII per il font GBA.

pub const ARGOMENTI: [(&str, &str); 4] = [
    ("MUOVERSI", "Usa su e giu per scegliere una voce."),
    ("FINESTRE", "Ogni app apre una finestra sulla scrivania."),
    ("MEMORIA", "Ogni app ha uno slot. Se manca spazio, l'avvio e negato."),
    ("SPEGNERE", "Chiudi le app e torna al menu per spegnere."),
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

    pub fn testo(&self) -> &'static str {
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
