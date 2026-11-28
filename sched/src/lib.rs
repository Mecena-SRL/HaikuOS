//! Scheduling a quote fisse per CPU (kernel-study riga 16). Puro, senza I/O e senza heap.
//! Il monolite dà a ogni task una quota di tick per periodo. Si sceglie round robin tra i
//! task pronti con quota residua; quando nessuno ne ha più, il periodo si ricarica.
//! Solo interi: nessun calcolo in virgola mobile.
#![cfg_attr(not(test), no_std)]

/// Identificatore di un task: indice dello slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Id(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Tutti gli slot sono occupati.
    Full,
    /// Slot non assegnato, oppure quota zero.
    NoTask,
}

#[derive(Clone, Copy)]
struct Slot {
    used: bool,
    ready: bool,
    quota: u32,
    left: u32,
}

const EMPTY: Slot = Slot { used: false, ready: false, quota: 0, left: 0 };

pub struct Sched<const N: usize> {
    slots: [Slot; N],
    cur: usize,
}

impl<const N: usize> Sched<N> {
    pub const fn new() -> Self {
        Sched { slots: [EMPTY; N], cur: 0 }
    }

    /// Assegna uno slot libero con `quota` tick per periodo. Il task parte non pronto.
    pub fn add(&mut self, quota: u32) -> Result<Id, Error> {
        if quota == 0 {
            return Err(Error::NoTask);
        }
        let i = self.slots.iter().position(|s| !s.used).ok_or(Error::Full)?;
        self.slots[i] = Slot { used: true, ready: false, quota, left: quota };
        Ok(Id(i as u8))
    }

    /// Libera lo slot.
    pub fn remove(&mut self, id: Id) -> Result<(), Error> {
        self.slot_mut(id).map(|s| *s = EMPTY)
    }

    pub fn set_ready(&mut self, id: Id, ready: bool) -> Result<(), Error> {
        self.slot_mut(id).map(|s| s.ready = ready)
    }

    /// Sceglie il prossimo task. Se nessun pronto ha quota, ricarica il periodo.
    /// `None` se nessun task è pronto.
    pub fn pick(&mut self) -> Option<Id> {
        if let Some(i) = self.find() {
            return Some(Id(i as u8));
        }
        let mut any = false;
        for s in self.slots.iter_mut().filter(|s| s.used) {
            s.left = s.quota;
            any |= s.ready;
        }
        if !any {
            return None;
        }
        self.find().map(|i| Id(i as u8))
    }

    /// Il task ha usato `ticks`. La quota residua non scende sotto zero.
    pub fn charge(&mut self, id: Id, ticks: u32) -> Result<(), Error> {
        self.slot_mut(id).map(|s| s.left = s.left.saturating_sub(ticks))
    }

    /// Quota residua nel periodo corrente.
    pub fn left(&self, id: Id) -> Option<u32> {
        self.slots.get(id.0 as usize).filter(|s| s.used).map(|s| s.left)
    }

    fn find(&mut self) -> Option<usize> {
        for k in 0..N {
            let i = (self.cur + k) % N;
            let s = &self.slots[i];
            if s.used && s.ready && s.left > 0 {
                self.cur = (i + 1) % N;
                return Some(i);
            }
        }
        None
    }

    fn slot_mut(&mut self, id: Id) -> Result<&mut Slot, Error> {
        match self.slots.get_mut(id.0 as usize) {
            Some(s) if s.used => Ok(s),
            _ => Err(Error::NoTask),
        }
    }
}

impl<const N: usize> Default for Sched<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_pieni() {
        let mut s: Sched<2> = Sched::new();
        assert!(s.add(1).is_ok());
        assert!(s.add(1).is_ok());
        assert_eq!(s.add(1), Err(Error::Full));
    }

    #[test]
    fn quota_zero_rifiutata() {
        let mut s: Sched<2> = Sched::new();
        assert_eq!(s.add(0), Err(Error::NoTask));
    }

    #[test]
    fn nessun_pronto_nessuna_scelta() {
        let mut s: Sched<2> = Sched::new();
        let a = s.add(2).unwrap();
        assert_eq!(s.pick(), None);
        s.set_ready(a, true).unwrap();
        assert_eq!(s.pick(), Some(a));
    }

    #[test]
    fn quote_rispettate() {
        // Periodo di 4 tick: A ha 3, B ha 1. Su 4 periodi: A 12 tick, B 4 tick.
        let mut s: Sched<4> = Sched::new();
        let a = s.add(3).unwrap();
        let b = s.add(1).unwrap();
        s.set_ready(a, true).unwrap();
        s.set_ready(b, true).unwrap();
        let (mut ca, mut cb) = (0, 0);
        for _ in 0..16 {
            let id = s.pick().unwrap();
            s.charge(id, 1).unwrap();
            if id == a { ca += 1 } else { cb += 1 }
        }
        assert_eq!((ca, cb), (12, 4));
    }

    #[test]
    fn task_non_pronto_saltato() {
        let mut s: Sched<3> = Sched::new();
        let a = s.add(5).unwrap();
        let b = s.add(5).unwrap();
        s.set_ready(b, true).unwrap();
        for _ in 0..5 {
            assert_eq!(s.pick(), Some(b));
            s.charge(b, 1).unwrap();
        }
        // B esaurito: il periodo si ricarica, ma A non è pronto e B sì.
        assert_eq!(s.pick(), Some(b));
        assert_eq!(s.left(a), Some(5));
    }

    #[test]
    fn rimozione_libera_slot() {
        let mut s: Sched<1> = Sched::new();
        let a = s.add(1).unwrap();
        assert_eq!(s.add(1), Err(Error::Full));
        s.remove(a).unwrap();
        assert_eq!(s.charge(a, 1), Err(Error::NoTask));
        assert!(s.add(1).is_ok());
    }
}
