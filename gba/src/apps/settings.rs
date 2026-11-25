//! Impostazioni: tema (4 palette 15 bit, vedi docs/desktop-concept.md) e nome del sistema.
//! Il primo tema è la palette del concept; gli altri tre sono provvisori (da scegliere).

pub const THEMES: usize = 4;
pub const NAME_MAX: usize = 12;

/// Palette per tema: T0 sfondo, T1 corpo, T2 titolo, T3 barra. Formato `r | g<<5 | b<<10`.
pub const PALETTES: [[u16; 4]; THEMES] = [
    [1 | 2 << 5 | 6 << 10, 31 | 31 << 5 | 31 << 10, 31 | 16 << 5, 0 | 20 << 5 | 31 << 10],
    [0 | 0 << 5 | 0 << 10, 31 | 31 << 5 | 31 << 10, 0 | 20 << 5 | 31 << 10, 31 | 16 << 5],
    [31 | 31 << 5 | 31 << 10, 1 | 2 << 5 | 6 << 10, 1 | 2 << 5 | 6 << 10, 0 | 20 << 5 | 31 << 10],
    [2 | 8 << 5 | 2 << 10, 31 | 31 << 5 | 31 << 10, 31 | 16 << 5, 0 | 20 << 5 | 31 << 10],
];

pub struct Settings {
    theme: usize,
    name: [u8; NAME_MAX],
    len: usize,
}

impl Settings {
    pub const fn new() -> Self {
        let mut name = [0u8; NAME_MAX];
        let def = b"HaikuOS";
        let mut i = 0;
        while i < def.len() {
            name[i] = def[i];
            i += 1;
        }
        Settings { theme: 0, name, len: def.len() }
    }

    pub fn theme(&self) -> usize {
        self.theme
    }

    /// Passa al tema successivo, con giro.
    pub fn next_theme(&mut self) {
        self.theme = (self.theme + 1) % THEMES;
    }

    pub fn prev_theme(&mut self) {
        self.theme = (self.theme + THEMES - 1) % THEMES;
    }

    pub fn palette(&self) -> [u16; 4] {
        PALETTES[self.theme]
    }

    pub fn nome(&self) -> &[u8] {
        &self.name[..self.len]
    }

    /// Aggiunge un carattere ASCII stampabile. `false` se pieno o non valido.
    pub fn push(&mut self, c: u8) -> bool {
        if self.len == NAME_MAX || !(0x20..=0x7E).contains(&c) {
            return false;
        }
        self.name[self.len] = c;
        self.len += 1;
        true
    }

    /// Cancella l'ultimo carattere. `false` se il nome è vuoto.
    pub fn pop(&mut self) -> bool {
        if self.len == 0 {
            return false;
        }
        self.len -= 1;
        true
    }
}
