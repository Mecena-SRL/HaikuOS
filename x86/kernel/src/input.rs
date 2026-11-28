//! Tastiera e mouse PS/2 in polling (niente IRQ). La decodifica sta in `haiku-ps2` (testata sull'host).
//! Mappa tasti: frecce = croce, Tab = cambia finestra, F4 = chiudi, Win = menu.
//! Lettere, cifre, spazio, Invio, Esc e Backspace arrivano come caratteri (ch).
//! Mouse: pacchetti di 3 byte; il tasto sinistro è il click e il trascinamento.
use crate::io::{inb, outb};
use haiku_gui::Key;
use haiku_ps2::{Decoder, Ev, Nav};

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
    dec: Decoder,
}

fn wait_write() {
    unsafe {
        for _ in 0..100_000 {
            if inb(STATUS) & 2 == 0 {
                return;
            }
        }
    }
}

impl Ps2 {
    pub fn init() -> Ps2 {
        unsafe {
            // Svuota il buffer, attiva il mouse ausiliario e il flusso dati.
            for _ in 0..16 {
                if inb(STATUS) & 1 == 0 {
                    break;
                }
                inb(DATA);
            }
            wait_write();
            outb(STATUS, 0xA8);
            wait_write();
            outb(STATUS, 0xD4);
            wait_write();
            outb(DATA, 0xF4);
        }
        Ps2 { dec: Decoder::new() }
    }

    /// Legge un byte se c'è; ritorna l'evento completo, se c'è.
    pub fn poll(&mut self) -> Option<Input> {
        let ev = unsafe {
            let st = inb(STATUS);
            if st & 1 == 0 {
                return None;
            }
            let b = inb(DATA);
            if st & 0x20 != 0 {
                self.dec.mouse(b)
            } else {
                self.dec.key(b)
            }
        }?;
        Some(match ev {
            Ev::Nav(n) => Input::Key {
                key: Some(match n {
                    Nav::Up => Key::Up,
                    Nav::Down => Key::Down,
                    Nav::Left => Key::Left,
                    Nav::Right => Key::Right,
                    Nav::Tab => Key::Select,
                    Nav::F4 => Key::B,
                }),
                ch: 0,
            },
            Ev::Char(ch) => Input::Key { key: None, ch },
            Ev::Super => Input::Super,
            Ev::Theme => Input::Theme,
            Ev::Mouse { dx, dy, left } => Input::Mouse { dx, dy, left },
        })
    }
}
