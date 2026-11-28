//! Editor di una riga con il solo keypad: su/giù cambia il carattere in attesa,
//! A lo inserisce alla posizione del cursore, B cancella il carattere prima del cursore,
//! L/R spostano il cursore.

use crate::glyph::CHARS;

pub const MAX: usize = 128;

pub struct Editor {
    buf: [u8; MAX],
    len: usize,
    cur: usize,
    pos: usize,
}

impl Editor {
    pub const fn new() -> Self {
        Editor { buf: [0; MAX], len: 0, cur: 0, pos: 0 }
    }

    /// Sostituisce il testo con `text` (tagliato a `MAX` byte). Il cursore va in fondo.
    pub fn load(&mut self, text: &[u8]) {
        let n = text.len().min(MAX);
        self.buf[..n].copy_from_slice(&text[..n]);
        self.len = n;
        self.cur = 0;
        self.pos = n;
    }

    pub fn text(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    /// Posizione del cursore, da 0 a `text().len()`.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Carattere in attesa, da inserire con `commit`.
    pub fn pending(&self) -> u8 {
        CHARS[self.cur]
    }

    pub fn up(&mut self) {
        self.cur = (self.cur + 1) % CHARS.len();
    }

    pub fn down(&mut self) {
        self.cur = (self.cur + CHARS.len() - 1) % CHARS.len();
    }

    pub fn left(&mut self) -> bool {
        if self.pos == 0 {
            return false;
        }
        self.pos -= 1;
        true
    }

    pub fn right(&mut self) -> bool {
        if self.pos == self.len {
            return false;
        }
        self.pos += 1;
        true
    }

    /// Inserisce il carattere in attesa alla posizione del cursore. Ritorna `false` se la riga è piena.
    pub fn commit(&mut self) -> bool {
        if self.len == MAX {
            return false;
        }
        self.buf.copy_within(self.pos..self.len, self.pos + 1);
        self.buf[self.pos] = self.pending();
        self.len += 1;
        self.pos += 1;
        true
    }

    /// Cancella il carattere prima del cursore. Ritorna `false` se il cursore è in testa.
    pub fn backspace(&mut self) -> bool {
        if self.pos == 0 {
            return false;
        }
        self.buf.copy_within(self.pos..self.len, self.pos - 1);
        self.len -= 1;
        self.pos -= 1;
        true
    }
}
