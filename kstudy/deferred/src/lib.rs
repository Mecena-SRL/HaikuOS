//! Lavoro differito (docs/kernel-study.md, riga 18): coda ad anello di dimensione fissa.
//! push() è chiamata dall'interrupt e non blocca; drain() esegue in ordine dal contesto normale.
//! Solo logica: con interrupt reali push e drain non devono sovrapporsi
//! (indici atomici oppure interrupt mascherati durante drain).
#![cfg_attr(not(test), no_std)]

/// Gestore: riceve il token dell'item.
pub type Handler = fn(u32);

#[derive(Clone, Copy)]
pub struct Item {
    pub token: u32,
    pub run: Handler,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Full;

pub struct Queue<const N: usize> {
    buf: [Option<Item>; N],
    head: usize,
    len: usize,
    drops: u32,
}

impl<const N: usize> Queue<N> {
    pub const fn new() -> Self {
        assert!(N > 0);
        Queue { buf: [None; N], head: 0, len: 0, drops: 0 }
    }

    /// Dall'interrupt: non blocca. Se la coda è piena conta un drop e ritorna Err.
    pub fn push(&mut self, item: Item) -> Result<(), Full> {
        if self.len == N {
            self.drops = self.drops.saturating_add(1);
            return Err(Full);
        }
        self.buf[(self.head + self.len) % N] = Some(item);
        self.len += 1;
        Ok(())
    }

    /// Dal contesto normale: esegue tutti gli item in ordine e svuota la coda.
    pub fn drain(&mut self) -> usize {
        let mut done = 0;
        while self.len > 0 {
            let Some(item) = self.buf[self.head].take() else { break };
            self.head = (self.head + 1) % N;
            self.len -= 1;
            (item.run)(item.token);
            done += 1;
        }
        done
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn drops(&self) -> u32 {
        self.drops
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        static LOG: RefCell<Vec<u32>> = RefCell::new(Vec::new());
    }

    fn log(token: u32) {
        LOG.with(|l| l.borrow_mut().push(token));
    }

    fn take_log() -> Vec<u32> {
        LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
    }

    fn item(token: u32) -> Item {
        Item { token, run: log }
    }

    #[test]
    fn order_preserved() {
        let mut q = Queue::<4>::new();
        for t in [7, 3, 9] {
            q.push(item(t)).unwrap();
        }
        assert_eq!(q.drain(), 3);
        assert_eq!(take_log(), vec![7, 3, 9]);
    }

    #[test]
    fn full_returns_err_and_counts_drop() {
        let mut q = Queue::<2>::new();
        q.push(item(1)).unwrap();
        q.push(item(2)).unwrap();
        assert_eq!(q.push(item(3)), Err(Full));
        assert_eq!(q.push(item(4)), Err(Full));
        assert_eq!(q.drops(), 2);
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn wraparound() {
        let mut q = Queue::<3>::new();
        q.push(item(1)).unwrap();
        q.push(item(2)).unwrap();
        q.drain();
        q.push(item(3)).unwrap();
        q.push(item(4)).unwrap();
        q.push(item(5)).unwrap(); // gli indici ora fanno il giro
        assert_eq!(q.push(item(6)), Err(Full));
        q.drain();
        assert_eq!(take_log(), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn drain_empties() {
        let mut q = Queue::<2>::new();
        q.push(item(8)).unwrap();
        q.push(item(9)).unwrap();
        assert_eq!(q.drain(), 2);
        assert_eq!(q.len(), 0);
        assert_eq!(q.drain(), 0);
        assert_eq!(take_log(), vec![8, 9]);
    }
}
