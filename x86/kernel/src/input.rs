//! Tastiera e mouse PS/2 in polling (niente IRQ). Tastiera: set 1 (default QEMU/PC).
//! Mappa tasti: frecce = croce, Tab = cambia finestra, F4 = chiudi, Win = menu.
//! Lettere, cifre, spazio, Invio, Esc e Backspace arrivano come caratteri (ch).
//! Mouse: pacchetti di 3 byte; il tasto sinistro è il click e il trascinamento.
use crate::io::{inb, outb};
use haiku_gui::Key;

const DATA: u16 = 0x60;
const STATUS: u16 = 0x64;

pub enum Input {
    /// `key` è il tasto della croce/A/B; `ch` il carattere (0 se nessuno).
    Key { key: Option<Key>, ch: u8 },
    /// Tasto Windows: apre o chiude il menu applicazioni.
    Super,
    /// F2: tema successivo.
    Theme,
    Mouse { dx: i32, dy: i32, left: bool },
}

pub struct Ps2 {
    pkt: [u8; 3],
    n: usize,
    e0: bool,
    shift: bool,
}

fn wait_write() {
    unsafe { while inb(STATUS) & 2 != 0 {} }
}

impl Ps2 {
    pub fn init() -> Ps2 {
        unsafe {
            // Svuota il buffer, attiva il mouse ausiliario e il flusso dati.
            while inb(STATUS) & 1 != 0 {
                inb(DATA);
            }
            wait_write();
            outb(STATUS, 0xA8);
            wait_write();
            outb(STATUS, 0xD4);
            wait_write();
            outb(DATA, 0xF4);
        }
        Ps2 { pkt: [0; 3], n: 0, e0: false, shift: false }
    }

    /// Legge un byte se c'è; ritorna l'evento completo, se c'è.
    pub fn poll(&mut self) -> Option<Input> {
        unsafe {
            let st = inb(STATUS);
            if st & 1 == 0 {
                return None;
            }
            let b = inb(DATA);
            if st & 0x20 != 0 {
                return self.mouse(b);
            }
            self.keyboard(b)
        }
    }

    fn keyboard(&mut self, b: u8) -> Option<Input> {
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
            return Some(Input::Theme);
        }
        if ext {
            let key = match b {
                0x5B => return Some(Input::Super),
                0x48 => Key::Up,
                0x50 => Key::Down,
                0x4B => Key::Left,
                0x4D => Key::Right,
                _ => return None,
            };
            return Some(Input::Key { key: Some(key), ch: 0 });
        }
        // Lettere e Invio sono testo (ch): nessuna lettera è un tasto di gioco.
        let key = match b {
            0x0F => Some(Key::Select), // Tab: cambia finestra
            0x3E => Some(Key::B),      // F4: chiude la finestra attiva
            _ => None,
        };
        let ch = ascii(b, self.shift);
        if key.is_none() && ch == 0 {
            return None;
        }
        Some(Input::Key { key, ch })
    }

    fn mouse(&mut self, b: u8) -> Option<Input> {
        // Ri-sincronizza: il primo byte ha il bit 3 a 1; ignora ACK (0xFA) e resto del self-test.
        if self.n == 0 && (b & 0x08 == 0 || b == 0xFA || b == 0xAA) {
            return None;
        }
        self.pkt[self.n] = b;
        self.n += 1;
        if self.n < 3 {
            return None;
        }
        self.n = 0;
        let f = self.pkt[0];
        let dx = self.pkt[1] as i8 as i32 - if f & 0x10 != 0 { 256 } else { 0 };
        let dy = self.pkt[2] as i8 as i32 - if f & 0x20 != 0 { 256 } else { 0 };
        Some(Input::Mouse { dx, dy, left: f & 1 != 0 })
    }
}

/// Carattere di un tasto del set 1 (layout US); 0 se non è stampabile né di controllo utile.
fn ascii(sc: u8, shift: bool) -> u8 {
    match sc {
        0x01 => 0x1B,                             // Esc
        0x09 if shift => b'*',
        0x02..=0x0A => b'1' + (sc - 0x02),        // 1..9
        0x0B => b'0',
        0x0C => b'-',
        0x0D if shift => b'+',
        0x0D => b'=',
        0x35 => b'/',
        0x0E => 0x08,                             // Backspace
        0x1C => 0x0D,                             // Invio
        0x10..=0x19 => b"qwertyuiop"[(sc - 0x10) as usize],
        0x1E..=0x26 => b"asdfghjkl"[(sc - 0x1E) as usize],
        0x2C..=0x32 => b"zxcvbnm"[(sc - 0x2C) as usize],
        0x39 => b' ',
        _ => 0,
    }
}
