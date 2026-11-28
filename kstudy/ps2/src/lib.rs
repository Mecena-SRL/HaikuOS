//! Decoder PS/2 tastiera (scan set 1), puro: nessuna I/O di porta, nessun interrupt.
//! Si alimenta un byte alla volta dalla porta dati (0x60) con `feed`, poi si svuota
//! la coda con `pop`. Nessuna allocazione: la coda è un anello di `QUEUE_CAP` eventi.
//!
//! Gestione:
//! - make = byte senza bit 7, break = make | 0x80.
//! - prefisso 0xE0 (tasti estesi), anche in break.
//! - 0xFA (ACK) e 0xFE (RESEND): ignorati, non producono eventi né cambiano stato.
//! - E1 (Pause, 6 byte in tutto): ignorato, i 5 byte successivi vengono saltati.
//! - E0 2A / E0 AA e E0 B7 (finti Shift di PrtSc): ignorati, resta il tasto esteso.
//! - Mouse PS/2 non incluso.
#![cfg_attr(not(test), no_std)]

/// Capacità della coda eventi (costante, nessuna allocazione).
pub const QUEUE_CAP: usize = 16;

const ACK: u8 = 0xFA;
const RESEND: u8 = 0xFE;

/// Un tasto premuto o rilasciato.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Event {
    /// Codice di scansione senza il bit di break.
    pub key: u8,
    /// `true` se preceduto da 0xE0.
    pub extended: bool,
    /// `true` per make, `false` per break.
    pub pressed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Idle,
    E0,
    /// Byte da saltare dopo E1 (sequenza Pause).
    Skip(u8),
}

#[derive(Debug)]
pub struct Decoder {
    state: State,
    buf: [Event; QUEUE_CAP],
    head: usize,
    len: usize,
    dropped: u32,
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder {
    pub const fn new() -> Self {
        Decoder {
            state: State::Idle,
            buf: [Event { key: 0, extended: false, pressed: false }; QUEUE_CAP],
            head: 0,
            len: 0,
            dropped: 0,
        }
    }

    /// Alimenta un byte letto dalla porta dati.
    pub fn feed(&mut self, b: u8) {
        if b == ACK || b == RESEND {
            return;
        }
        match self.state {
            State::Skip(n) => self.state = if n > 1 { State::Skip(n - 1) } else { State::Idle },
            State::Idle if b == 0xE0 => self.state = State::E0,
            State::Idle if b == 0xE1 => self.state = State::Skip(5),
            State::Idle => self.emit(b, false),
            // Finti Shift di PrtSc: il tasto vero arriva dopo, con un altro E0.
            State::E0 if b == 0x2A || b == 0xAA => self.state = State::Idle,
            State::E0 if b == 0xE0 => {}
            State::E0 => {
                self.state = State::Idle;
                self.emit(b, true);
            }
        }
    }

    /// Prossimo evento in coda, se c'è.
    pub fn pop(&mut self) -> Option<Event> {
        if self.len == 0 {
            return None;
        }
        let e = self.buf[self.head];
        self.head = (self.head + 1) % QUEUE_CAP;
        self.len -= 1;
        Some(e)
    }

    /// Eventi scartati perché la coda era piena.
    pub fn dropped(&self) -> u32 {
        self.dropped
    }

    fn emit(&mut self, b: u8, extended: bool) {
        if self.len == QUEUE_CAP {
            self.dropped = self.dropped.saturating_add(1);
            return;
        }
        self.buf[(self.head + self.len) % QUEUE_CAP] =
            Event { key: b & 0x7F, extended, pressed: b & 0x80 == 0 };
        self.len += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed_all(d: &mut Decoder, bytes: &[u8]) {
        for &b in bytes {
            d.feed(b);
        }
    }

    #[test]
    fn letter_make_and_break() {
        let mut d = Decoder::new();
        feed_all(&mut d, &[0x1E, 0x9E]); // 'a' make, break
        assert_eq!(d.pop(), Some(Event { key: 0x1E, extended: false, pressed: true }));
        assert_eq!(d.pop(), Some(Event { key: 0x1E, extended: false, pressed: false }));
        assert_eq!(d.pop(), None);
    }

    #[test]
    fn extended_arrow_left() {
        let mut d = Decoder::new();
        feed_all(&mut d, &[0xE0, 0x4B, 0xE0, 0xCB]); // Left make, Left break
        assert_eq!(d.pop(), Some(Event { key: 0x4B, extended: true, pressed: true }));
        assert_eq!(d.pop(), Some(Event { key: 0x4B, extended: true, pressed: false }));
        assert_eq!(d.pop(), None);
    }

    #[test]
    fn extended_arrow_up_split_across_calls() {
        let mut d = Decoder::new();
        d.feed(0xE0);
        assert_eq!(d.pop(), None); // prefisso: nessun evento ancora
        d.feed(0x48);
        assert_eq!(d.pop(), Some(Event { key: 0x48, extended: true, pressed: true }));
        assert_eq!(d.pop(), None);
    }

    #[test]
    fn ack_and_resend_are_ignored() {
        let mut d = Decoder::new();
        feed_all(&mut d, &[0xFA, 0xFE, 0x1E]);
        assert_eq!(d.pop(), Some(Event { key: 0x1E, extended: false, pressed: true }));
        assert_eq!(d.pop(), None);
        assert_eq!(d.dropped(), 0);
    }

    #[test]
    fn pause_sequence_is_skipped() {
        let mut d = Decoder::new();
        feed_all(&mut d, &[0xE1, 0x1D, 0x45, 0xE1, 0x9D, 0xC5, 0x1E]);
        assert_eq!(d.pop(), Some(Event { key: 0x1E, extended: false, pressed: true }));
        assert_eq!(d.pop(), None);
    }

    #[test]
    fn print_screen_fake_shift_is_skipped() {
        let mut d = Decoder::new();
        feed_all(&mut d, &[0xE0, 0x2A, 0xE0, 0x37]); // PrtSc make
        assert_eq!(d.pop(), Some(Event { key: 0x37, extended: true, pressed: true }));
        assert_eq!(d.pop(), None);
    }

    #[test]
    fn queue_overflow_counts_drops() {
        let mut d = Decoder::new();
        for _ in 0..QUEUE_CAP + 4 {
            d.feed(0x1E);
        }
        assert_eq!(d.dropped(), 4);
        let mut n = 0;
        while d.pop().is_some() {
            n += 1;
        }
        assert_eq!(n, QUEUE_CAP);
        // Dopo lo svuotamento la coda riprende a riempirsi.
        d.feed(0x1E);
        assert!(d.pop().is_some());
        assert_eq!(d.dropped(), 4);
    }
}
