//! Decodifica PS/2 (i8042) senza I/O: set 1 per la tastiera, pacchetti da 3 byte per il mouse.
//! Il kernel legge le porte e passa qui i byte; il codice è puro e si testa sull'host.
#![cfg_attr(not(test), no_std)]

/// Tasti di navigazione e di sistema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nav {
    Up,
    Down,
    Left,
    Right,
    /// Tab: cambia finestra.
    Tab,
    /// F4: chiude la finestra attiva.
    F4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ev {
    Nav(Nav),
    /// Carattere ASCII (lettere, cifre, Invio, Esc, Backspace, spazio...).
    Char(u8),
    /// Tasto Windows: apre o chiude il menu.
    Super,
    /// F2: tema successivo.
    Theme,
    Mouse { dx: i32, dy: i32, left: bool },
}

#[derive(Debug, Default)]
pub struct Decoder {
    pkt: [u8; 3],
    n: usize,
    e0: bool,
    shift: bool,
}

impl Decoder {
    pub const fn new() -> Self {
        Self { pkt: [0; 3], n: 0, e0: false, shift: false }
    }

    /// Un byte dalla tastiera (set 1). `None` se il byte non produce un evento.
    pub fn key(&mut self, b: u8) -> Option<Ev> {
        if b == 0xE0 {
            self.e0 = true;
            return None;
        }
        let ext = self.e0;
        self.e0 = false;
        // Shift sinistro/destro: serve per + e *.
        if !ext && (b == 0x2A || b == 0x36) {
            self.shift = true;
            return None;
        }
        if !ext && (b == 0xAA || b == 0xB6) {
            self.shift = false;
            return None;
        }
        if b & 0x80 != 0 {
            return None; // rilascio
        }
        if !ext && b == 0x3C {
            return Some(Ev::Theme);
        }
        if ext {
            let nav = match b {
                0x5B => return Some(Ev::Super),
                0x48 => Nav::Up,
                0x50 => Nav::Down,
                0x4B => Nav::Left,
                0x4D => Nav::Right,
                _ => return None,
            };
            return Some(Ev::Nav(nav));
        }
        match b {
            0x0F => return Some(Ev::Nav(Nav::Tab)),
            0x3E => return Some(Ev::Nav(Nav::F4)),
            _ => {}
        }
        match ascii(b, self.shift) {
            0 => None,
            ch => Some(Ev::Char(ch)),
        }
    }

    /// Un byte dal mouse: pacchetti da 3 byte, risincronizzati sul bit 3 del primo.
    pub fn mouse(&mut self, b: u8) -> Option<Ev> {
        // Ignora ACK (0xFA), resto del self-test (0xAA) e byte senza il bit 3 all'inizio.
        if self.n == 0 && (b & 0x08 == 0 || b == 0xFA || b == 0xAA) {
            return None;
        }
        self.pkt[self.n] = b;
        self.n += 1;
        if self.n < 3 {
            return None;
        }
        self.n = 0;
        // Il byte è già con segno (i8): il bit di segno nel primo byte non va sommato di nuovo.
        let f = self.pkt[0];
        Some(Ev::Mouse {
            dx: self.pkt[1] as i8 as i32,
            dy: self.pkt[2] as i8 as i32,
            left: f & 1 != 0,
        })
    }
}

/// Carattere di un tasto del set 1 (layout US); 0 se non è stampabile né di controllo utile.
fn ascii(sc: u8, shift: bool) -> u8 {
    match sc {
        0x01 => 0x1B,                      // Esc
        0x09 if shift => b'*',
        0x02..=0x0A => b'1' + (sc - 0x02), // 1..9
        0x0B => b'0',
        0x0C => b'-',
        0x0D if shift => b'+',
        0x0D => b'=',
        0x3F => 0x05,                      // F5: esegue lo script nell'editor
        0x35 => b'/',
        0x0E => 0x08,                      // Backspace
        0x1C => 0x0D,                      // Invio
        0x10..=0x19 => b"qwertyuiop"[(sc - 0x10) as usize],
        0x1E..=0x26 => b"asdfghjkl"[(sc - 0x1E) as usize],
        0x2C..=0x32 => b"zxcvbnm"[(sc - 0x2C) as usize],
        0x39 => b' ',
        _ => 0,
    }
}

/// Sorgente di byte PS/2. Adattatori: porta 0x60 nel kernel, slice registrato nei test host.
pub trait ByteSource {
    fn next_byte(&mut self) -> Option<u8>;
}

/// Porta la sorgente fino a esaurirla; ogni evento va al `sink`. Non alloca.
/// `mouse` indica se i byte sono pacchetti mouse (3 byte) invece di tastiera.
pub fn pump<S: ByteSource, F: FnMut(Ev)>(dec: &mut Decoder, src: &mut S, mouse: bool, mut sink: F) {
    while let Some(b) = src.next_byte() {
        let ev = if mouse { dec.mouse(b) } else { dec.key(b) };
        if let Some(ev) = ev {
            sink(ev);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lettere_e_rilascio() {
        let mut d = Decoder::new();
        assert_eq!(d.key(0x1E), Some(Ev::Char(b'a')));
        assert_eq!(d.key(0x9E), None);
    }

    #[test]
    fn shift_per_piu_e_asterisco() {
        let mut d = Decoder::new();
        assert_eq!(d.key(0x2A), None);
        assert_eq!(d.key(0x0D), Some(Ev::Char(b'+')));
        assert_eq!(d.key(0xAA), None);
        assert_eq!(d.key(0x0D), Some(Ev::Char(b'=')));
    }

    #[test]
    fn tasti_estesi_e_di_sistema() {
        let mut d = Decoder::new();
        assert_eq!(d.key(0xE0), None);
        assert_eq!(d.key(0x48), Some(Ev::Nav(Nav::Up)));
        assert_eq!(d.key(0xE0), None);
        assert_eq!(d.key(0x5B), Some(Ev::Super));
        assert_eq!(d.key(0x3C), Some(Ev::Theme));
        assert_eq!(d.key(0x0F), Some(Ev::Nav(Nav::Tab)));
        assert_eq!(d.key(0x3E), Some(Ev::Nav(Nav::F4)));
        assert_eq!(d.key(0x1C), Some(Ev::Char(0x0D)));
    }

    #[test]
    fn mouse_positivo_e_negativo() {
        let mut d = Decoder::new();
        assert_eq!(d.mouse(0xFA), None); // ACK ignorato
        assert_eq!(d.mouse(0x09), None);
        assert_eq!(d.mouse(0x05), None);
        assert_eq!(d.mouse(0x00), Some(Ev::Mouse { dx: 5, dy: 0, left: true }));
        // dx = -2 (0xFE), dy = -3 (0xFD), bit di segno X e Y, tasto destro: sinistro assente.
        assert_eq!(d.mouse(0x38), None);
        assert_eq!(d.mouse(0xFE), None);
        assert_eq!(d.mouse(0xFD), Some(Ev::Mouse { dx: -2, dy: -3, left: false }));
    }
}
