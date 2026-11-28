//! Coda di eventi a dimensione fissa con timestamp (docs/kernel-study.md, riga 8).
//! Nessuna allocazione: l'anello vive nella struttura. Se la coda è piena il nuovo evento
//! viene scartato e contato, così il produttore (interrupt o polling) non si blocca mai.
#![cfg_attr(not(test), no_std)]

/// Evento con il tempo di arrivo (tick monotoni del sistema).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timed<T> {
    pub ts: u64,
    pub ev: T,
}

pub struct Queue<T: Copy, const N: usize> {
    buf: [Option<Timed<T>>; N],
    head: usize,
    len: usize,
    dropped: u32,
}

impl<T: Copy, const N: usize> Queue<T, N> {
    pub const fn new() -> Self {
        Self { buf: [None; N], head: 0, len: 0, dropped: 0 }
    }

    /// Accoda un evento. `false` se la coda è piena (evento scartato e contato).
    pub fn push(&mut self, ts: u64, ev: T) -> bool {
        if self.len == N {
            self.dropped = self.dropped.saturating_add(1);
            return false;
        }
        let i = (self.head + self.len) % N;
        self.buf[i] = Some(Timed { ts, ev });
        self.len += 1;
        true
    }

    /// Estrae il primo evento (ordine di arrivo).
    pub fn pop(&mut self) -> Option<Timed<T>> {
        if self.len == 0 {
            return None;
        }
        let out = self.buf[self.head].take();
        self.head = (self.head + 1) % N;
        self.len -= 1;
        out
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Eventi scartati da quando la coda è stata creata.
    pub fn dropped(&self) -> u32 {
        self.dropped
    }
}

impl<T: Copy, const N: usize> Default for Queue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordine_e_tempo() {
        let mut q: Queue<u8, 4> = Queue::new();
        assert!(q.push(10, 1));
        assert!(q.push(12, 2));
        assert_eq!(q.pop(), Some(Timed { ts: 10, ev: 1 }));
        assert_eq!(q.pop(), Some(Timed { ts: 12, ev: 2 }));
        assert_eq!(q.pop(), None);
        assert!(q.is_empty());
    }

    #[test]
    fn avvolgimento_dell_anello() {
        let mut q: Queue<u8, 3> = Queue::new();
        for round in 0..10u8 {
            assert!(q.push(round as u64, round));
            assert_eq!(q.pop().map(|t| t.ev), Some(round));
        }
        assert!(q.is_empty());
    }

    #[test]
    fn coda_piena_scarta_e_conta() {
        let mut q: Queue<u8, 2> = Queue::new();
        assert!(q.push(1, 1));
        assert!(q.push(2, 2));
        assert!(!q.push(3, 3));
        assert!(!q.push(4, 4));
        assert_eq!(q.dropped(), 2);
        assert_eq!(q.len(), 2);
        assert_eq!(q.pop().map(|t| t.ev), Some(1));
        assert!(q.push(5, 5));
        assert_eq!(q.pop().map(|t| t.ev), Some(2));
        assert_eq!(q.pop().map(|t| t.ev), Some(5));
    }
}
