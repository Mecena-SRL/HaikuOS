//! Orologio: ora su 24 ore, avanzato dai frame del GBA.
//! Usa 60 frame al secondo; il GBA ne dà ~59,73 (errore da misurare: da verificare).

pub const FPS: u64 = 60;
const DAY: u32 = 86_400;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    /// Secondi dalla mezzanotte, 0..86400.
    secs: u32,
    /// Frame residui oltre l'ultimo secondo intero, 0..60.
    frames: u64,
}

impl Clock {
    pub const fn new(h: u8, m: u8) -> Self {
        Clock { secs: (h as u32 % 24) * 3600 + (m as u32 % 60) * 60, frames: 0 }
    }

    /// Imposta l'ora e azzera i secondi residui.
    pub fn set(&mut self, h: u8, m: u8) {
        *self = Clock::new(h, m);
    }

    /// Avanza di `n` frame.
    pub fn tick(&mut self, n: u32) {
        let total = self.frames + n as u64;
        self.frames = total % FPS;
        let secs = (total / FPS) as u32;
        self.secs = (self.secs + secs % DAY) % DAY;
    }

    /// (ore, minuti, secondi).
    pub fn hms(&self) -> (u8, u8, u8) {
        let h = (self.secs / 3600) as u8;
        let m = ((self.secs % 3600) / 60) as u8;
        let s = (self.secs % 60) as u8;
        (h, m, s)
    }

    /// Testo "HH:MM" in ASCII.
    pub fn testo(&self) -> [u8; 5] {
        let (h, m, _) = self.hms();
        [b'0' + h / 10, b'0' + h % 10, b':', b'0' + m / 10, b'0' + m % 10]
    }
}
