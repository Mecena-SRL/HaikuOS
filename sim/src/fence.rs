//! Fence a contatore monotono per dispositivo (idea Linux dma-fence, riscritta da zero).
//! Ogni invio prende un numero di sequenza; il dispositivo segnala il completamento
//! alzando il contatore. Un numero è completato quando il contatore lo ha raggiunto.
//! Nessun hardware reale. Regole: docs/kernel-study.md, riga 23.

#[derive(Debug, Default)]
pub struct Fence {
    next: u64,
    done: u64,
}

impl Fence {
    pub fn new() -> Self {
        Self::default()
    }

    /// Numero di sequenza per il prossimo invio (da 1 in poi; 0 = nulla in attesa).
    pub fn next_seq(&mut self) -> u64 {
        self.next += 1;
        self.next
    }

    /// Lato dispositivo: segnala il completamento fino a `seq`. Non torna mai indietro.
    pub fn signal(&mut self, seq: u64) {
        self.done = self.done.max(seq.min(self.next));
    }

    pub fn is_done(&self, seq: u64) -> bool {
        seq <= self.done
    }

    pub fn completed(&self) -> u64 {
        self.done
    }
}
