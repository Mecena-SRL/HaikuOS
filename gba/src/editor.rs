//! Editor di una riga con il solo keypad: su/giù cambia il carattere in attesa,
//! A lo aggiunge, B cancella l'ultimo. Nessuna shell, nessun input di testo libero.

use crate::glyph::CHARS;

pub const MAX: usize = 24;

pub struct Editor {
    buf: [u8; MAX],
    len: usize,
    cur: usize,
}

impl Editor {
    pub const fn new() -> Self {
        Editor { buf: [0; MAX], len: 0, cur: 0 }
    }

    /// Sostituisce il testo con `text` (tagliato a `MAX` byte) e azzera il carattere in attesa.
    pub fn load(&mut self, text: &[u8]) {
        let n = text.len().min(MAX);
        self.buf[..n].copy_from_slice(&text[..n]);
        self.len = n;
        self.cur = 0;
    }

    pub fn text(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    /// Carattere in attesa, da aggiungere con `commit`.
    pub fn pending(&self) -> u8 {
        CHARS[self.cur]
    }

    pub fn up(&mut self) {
        self.cur = (self.cur + 1) % CHARS.len();
    }

    pub fn down(&mut self) {
        self.cur = (self.cur + CHARS.len() - 1) % CHARS.len();
    }

    /// Aggiunge il carattere in attesa. Ritorna `false` se la riga è piena.
    pub fn commit(&mut self) -> bool {
        if self.len == MAX {
            return false;
        }
        self.buf[self.len] = self.pending();
        self.len += 1;
        true
    }

    /// Cancella l'ultimo carattere. Ritorna `false` se la riga è vuota.
    pub fn backspace(&mut self) -> bool {
        if self.len == 0 {
            return false;
        }
        self.len -= 1;
        true
    }
}
