//! Promemoria del desktop: lista a capacità fissa (senza heap), con stato "fatto" e cancellazione.
//! Testo maiuscolo, fino a TEXT_MAX caratteri (il font ha solo alcune lettere).

pub const SLOTS: usize = 4;
pub const TEXT_MAX: usize = 14;

#[derive(Clone, Copy)]
pub struct Item {
    pub text: [u8; TEXT_MAX],
    pub len: usize,
    pub done: bool,
}

pub struct Reminders {
    items: [Option<Item>; SLOTS],
    pub sel: usize,
}

impl Reminders {
    pub const fn new() -> Self {
        Self { items: [None; SLOTS], sel: 0 }
    }

    /// Aggiunge un promemoria nel primo posto libero. False se la lista è piena.
    pub fn add(&mut self, text: &[u8]) -> bool {
        let slot = match self.items.iter().position(|i| i.is_none()) {
            Some(s) => s,
            None => return false,
        };
        let mut t = [0u8; TEXT_MAX];
        let len = text.len().min(TEXT_MAX);
        t[..len].copy_from_slice(&text[..len]);
        self.items[slot] = Some(Item { text: t, len, done: false });
        true
    }

    /// Cambia lo stato "fatto" della voce selezionata.
    pub fn toggle(&mut self) {
        if let Some(it) = self.items[self.sel].as_mut() {
            it.done = !it.done;
        }
    }

    /// Cancella la voce selezionata.
    pub fn remove(&mut self) {
        self.items[self.sel] = None;
    }

    /// Muove la selezione, con ritorno da fine a inizio.
    pub fn step(&mut self, delta: isize) {
        self.sel = (self.sel as isize + delta).rem_euclid(SLOTS as isize) as usize;
    }

    pub fn item(&self, i: usize) -> Option<&Item> {
        self.items.get(i).and_then(|o| o.as_ref())
    }

    /// Voci presenti (fatte o no).
    pub fn filled(&self) -> usize {
        self.items.iter().flatten().count()
    }

    /// Voci non ancora fatte.
    pub fn open(&self) -> usize {
        self.items.iter().flatten().filter(|i| !i.done).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_toggle_remove() {
        let mut r = Reminders::new();
        assert!(r.add(b"TELEFONA ANNA"));
        assert_eq!(r.open(), 1);
        r.toggle();
        assert_eq!(r.open(), 0);
        assert!(r.item(0).unwrap().done);
        r.remove();
        assert!(r.item(0).is_none());
    }

    #[test]
    fn full_list_refuses_and_text_is_cut() {
        let mut r = Reminders::new();
        for _ in 0..SLOTS {
            assert!(r.add(b"LEGGI MAIL"));
        }
        assert!(!r.add(b"NUOVO"));
        let mut r2 = Reminders::new();
        r2.add(b"UNTESTOMOLTOLUNGOCHEVAOLTRE");
        assert_eq!(r2.item(0).unwrap().len, TEXT_MAX);
    }

    #[test]
    fn selection_wraps() {
        let mut r = Reminders::new();
        r.step(-1);
        assert_eq!(r.sel, SLOTS - 1);
        r.step(1);
        assert_eq!(r.sel, 0);
    }
}
