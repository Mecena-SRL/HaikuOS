//! Anello di comandi a dimensione fissa con campanello (idea Linux amdgpu/virtio-gpu, riscritta).
//! Il programma scrive in coda e suona il campanello; il dispositivo consuma fino al campanello.
//! Indici a 32 bit con avvolgimento: pieno quando `tail - head == N`. Nessun heap.
//! Nessun hardware reale. Regole: docs/kernel-study.md, riga 22 (rinviata: solo prototipo).

#[derive(Debug, PartialEq, Eq)]
pub enum RingError {
    Full,
}

pub struct Ring<T: Copy + Default, const N: usize> {
    slots: [T; N],
    head: u32,
    tail: u32,
    rung: u32,
}

impl<T: Copy + Default, const N: usize> Ring<T, N> {
    pub fn new() -> Self {
        assert!(N > 0 && N.is_power_of_two(), "anello: N deve essere potenza di due");
        Self { slots: [T::default(); N], head: 0, tail: 0, rung: 0 }
    }

    /// Lato programma: scrive un comando senza attendere.
    pub fn push(&mut self, cmd: T) -> Result<(), RingError> {
        if self.tail.wrapping_sub(self.head) as usize >= N {
            return Err(RingError::Full);
        }
        self.slots[self.tail as usize % N] = cmd;
        self.tail = self.tail.wrapping_add(1);
        Ok(())
    }

    /// Lato programma: suona il campanello, il dispositivo vede i nuovi comandi.
    pub fn ring(&mut self) {
        self.rung = self.tail;
    }

    /// Lato dispositivo: consuma fino al campanello, chiamando `exec` per ogni comando.
    /// Ritorna quanti comandi ha eseguiti.
    pub fn consume(&mut self, mut exec: impl FnMut(T)) -> usize {
        let mut n = 0;
        while self.head != self.rung {
            exec(self.slots[self.head as usize % N]);
            self.head = self.head.wrapping_add(1);
            n += 1;
        }
        n
    }

    pub fn pending(&self) -> usize {
        self.tail.wrapping_sub(self.head) as usize
    }
}
