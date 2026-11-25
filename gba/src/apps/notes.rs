//! App Note: fino a `N` note di una riga, ognuna con un `Editor` (riusato da `editor.rs`).
//! Logica pura: niente disegno, niente salvataggio. Il `dirty` dice se c'è da salvare.

use crate::editor::Editor;

pub const N: usize = 8;

pub struct Notes {
    items: [Editor; N],
    len: usize,
    cur: usize,
    dirty: bool,
}

impl Notes {
    pub const fn new() -> Self {
        Notes { items: [const { Editor::new() }; N], len: 0, cur: 0, dirty: false }
    }

    pub fn count(&self) -> usize {
        self.len
    }

    pub fn selected(&self) -> usize {
        self.cur
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Segna le note come salvate.
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Crea una nota vuota e la seleziona. `false` se le note sono piene.
    pub fn nuova(&mut self) -> bool {
        if self.len == N {
            return false;
        }
        self.items[self.len].load(b"");
        self.cur = self.len;
        self.len += 1;
        self.dirty = true;
        true
    }

    /// Seleziona la nota `i`. `false` se non esiste.
    pub fn seleziona(&mut self, i: usize) -> bool {
        if i >= self.len {
            return false;
        }
        self.cur = i;
        true
    }

    /// Carica il testo di una nota esistente (es. dal file system). Non sporca lo stato.
    pub fn carica(&mut self, i: usize, testo: &[u8]) -> bool {
        if i >= self.len {
            return false;
        }
        self.items[i].load(testo);
        true
    }

    /// Carattere in attesa della nota selezionata (spazio se non ci sono note).
    pub fn pending(&self) -> u8 {
        if self.len == 0 { b' ' } else { self.items[self.cur].pending() }
    }

    pub fn testo(&self) -> &[u8] {
        if self.len == 0 {
            return b"";
        }
        self.items[self.cur].text()
    }

    /// Aggiunge il carattere in attesa alla nota selezionata.
    pub fn commit(&mut self) -> bool {
        if self.len == 0 || !self.items[self.cur].commit() {
            return false;
        }
        self.dirty = true;
        true
    }

    /// Cancella l'ultimo carattere della nota selezionata.
    pub fn backspace(&mut self) -> bool {
        if self.len == 0 || !self.items[self.cur].backspace() {
            return false;
        }
        self.dirty = true;
        true
    }

    /// Scorre il carattere in attesa (su/giù) della nota selezionata.
    pub fn up(&mut self) {
        if self.len > 0 {
            self.items[self.cur].up();
        }
    }

    pub fn down(&mut self) {
        if self.len > 0 {
            self.items[self.cur].down();
        }
    }

    /// Testo della nota `i` (vuoto se non esiste).
    pub fn testo_di(&self, i: usize) -> &[u8] {
        if i >= self.len { b"" } else { self.items[i].text() }
    }

    /// Posizione del cursore nella nota selezionata.
    pub fn pos(&self) -> usize {
        if self.len == 0 { 0 } else { self.items[self.cur].pos() }
    }

    pub fn left(&mut self) -> bool {
        self.len > 0 && self.items[self.cur].left()
    }

    pub fn right(&mut self) -> bool {
        self.len > 0 && self.items[self.cur].right()
    }

    /// Seleziona la nota precedente (-1) o successiva (+1), senza giro.
    pub fn scorri(&mut self, delta: i32) -> bool {
        let nuovo = self.cur as i32 + delta;
        if nuovo < 0 || nuovo >= self.len as i32 {
            return false;
        }
        self.cur = nuovo as usize;
        true
    }
}
