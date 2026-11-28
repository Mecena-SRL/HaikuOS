//! Tracce a buffer circolare a dimensione fissa: nessuna allocazione, nessun orologio.
//! Ogni evento ha un'etichetta numerica (u16), un argomento (u32) e un numero di sequenza.
//! Quando il buffer è pieno si sovrascrive il più vecchio e si conta la perdita.
#![cfg_attr(not(test), no_std)]

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Event {
    pub seq: u32,
    pub label: u16,
    pub arg: u32,
}

pub struct Ring<const N: usize> {
    buf: [Event; N],
    next_seq: u32,
    overwritten: u32,
}

impl<const N: usize> Ring<N> {
    pub const fn new() -> Self {
        Self { buf: [Event { seq: 0, label: 0, arg: 0 }; N], next_seq: 0, overwritten: 0 }
    }

    /// Registra un evento. Sovrascrive il più vecchio se pieno.
    pub fn record(&mut self, label: u16, arg: u32) {
        if N == 0 {
            return;
        }
        let seq = self.next_seq;
        if seq as usize >= N {
            self.overwritten += 1;
        }
        self.buf[seq as usize % N] = Event { seq, label, arg };
        self.next_seq = seq.wrapping_add(1);
    }

    /// Eventi ancora presenti, dal più vecchio al più recente.
    pub fn iter(&self) -> impl Iterator<Item = Event> + '_ {
        let total = self.next_seq as usize;
        let kept = total.min(N);
        let start = total - kept;
        (start..total).map(move |i| self.buf[i % N])
    }

    /// Quanti eventi sono stati sovrascritti prima di essere letti.
    pub fn overwritten(&self) -> u32 {
        self.overwritten
    }
}

impl<const N: usize> Default for Ring<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordine_prima_del_giro() {
        let mut r: Ring<4> = Ring::new();
        r.record(1, 10);
        r.record(2, 20);
        let v: Vec<_> = r.iter().map(|e| (e.label, e.arg)).collect();
        assert_eq!(v, vec![(1, 10), (2, 20)]);
        assert_eq!(r.overwritten(), 0);
    }

    #[test]
    fn sovrascrive_il_piu_vecchio() {
        let mut r: Ring<3> = Ring::new();
        for i in 0..5u32 {
            r.record(i as u16, i);
        }
        let v: Vec<_> = r.iter().map(|e| e.arg).collect();
        assert_eq!(v, vec![2, 3, 4]);
        assert_eq!(r.overwritten(), 2);
        assert_eq!(r.iter().next().unwrap().seq, 2);
    }

    #[test]
    fn capacita_zero_non_panica() {
        let mut r: Ring<0> = Ring::new();
        r.record(1, 1);
        assert_eq!(r.iter().count(), 0);
    }
}
