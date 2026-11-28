//! Lavoro differito dopo l'interrupt (docs/kernel-study.md, riga 18): il gestore copre solo
//! il minimo e accoda una funzione con un argomento. Coda fissa, nessuna allocazione;
//! il lavoro gira fuori dal gestore, quando il kernel chiama `run_pending`.
//! Un lavoro in più sulla coda piena si scarta e si conta.

use crate::resource::{Error, Queue};

/// Funzione differita: riceve l'argomento accodato.
pub type Work = fn(u32);

#[derive(Clone, Copy)]
struct Item {
    run: Work,
    arg: u32,
}

pub struct Deferred {
    queue: Queue<Item>,
    dropped: u32,
}

impl Deferred {
    pub fn new(depth: usize) -> Self {
        Self { queue: Queue::new(depth), dropped: 0 }
    }

    /// Chiamabile dal gestore: non esegue nulla, solo accoda. `Full` se la coda è piena.
    pub fn push(&mut self, run: Work, arg: u32) -> Result<(), Error> {
        let r = self.queue.push(Item { run, arg });
        if r.is_err() {
            self.dropped += 1;
        }
        r
    }

    /// Esegue fino a `max` lavori in ordine FIFO; ritorna quanti ne ha eseguiti.
    pub fn run_pending(&mut self, max: usize) -> usize {
        let mut n = 0;
        while n < max {
            match self.queue.pop() {
                Some(it) => (it.run)(it.arg),
                None => break,
            }
            n += 1;
        }
        n
    }

    pub fn dropped(&self) -> u32 {
        self.dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static LOG: AtomicU32 = AtomicU32::new(0);

    fn somma(arg: u32) {
        LOG.fetch_add(arg, Ordering::SeqCst);
    }

    #[test]
    fn esegue_in_ordine_e_rispetta_il_limite() {
        let mut d = Deferred::new(4);
        d.push(somma, 1).unwrap();
        d.push(somma, 10).unwrap();
        d.push(somma, 100).unwrap();
        assert_eq!(d.run_pending(2), 2);
        assert_eq!(d.run_pending(8), 1);
        assert_eq!(d.run_pending(8), 0);
        assert_eq!(LOG.load(Ordering::SeqCst), 111);
    }

    #[test]
    fn coda_piena_scarta_e_conta() {
        let mut d = Deferred::new(1);
        assert!(d.push(somma, 1).is_ok());
        assert_eq!(d.push(somma, 2), Err(Error::Full));
        assert_eq!(d.dropped(), 1);
    }
}
