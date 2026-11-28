//! Quote di tempo per task (idea 16 di docs/kernel-study.md).
//! Tabella fissa di N task; ogni task ha una quota per periodo e un contatore di uso.
//! Il tetto della quota è approvato dal monolite (`CEILING`). Solo interi, nessuna allocazione.
#![cfg_attr(not(test), no_std)]

/// Quota massima per periodo, in unità di tempo. Valore da allineare al limite del monolite.
pub const CEILING: u32 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Tabella piena (N task).
    Full,
    /// Id inesistente o task non attivo.
    NoSuchTask,
    /// Quota sopra `CEILING`.
    OverCeiling,
}

#[derive(Clone, Copy)]
struct Slot {
    active: bool,
    quota: u32,
    used: u32,
}

pub struct Quota<const N: usize> {
    slots: [Slot; N],
    /// Dove riprende la ricerca round-robin.
    next: usize,
}

impl<const N: usize> Default for Quota<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Quota<N> {
    pub const fn new() -> Self {
        Quota { slots: [Slot { active: false, quota: 0, used: 0 }; N], next: 0 }
    }

    /// Aggiunge un task con quota 0. Restituisce il suo id.
    pub fn add(&mut self) -> Result<usize, Error> {
        let id = self.slots.iter().position(|s| !s.active).ok_or(Error::Full)?;
        self.slots[id].active = true;
        Ok(id)
    }

    fn slot(&mut self, task: usize) -> Result<&mut Slot, Error> {
        match self.slots.get_mut(task) {
            Some(s) if s.active => Ok(s),
            _ => Err(Error::NoSuchTask),
        }
    }

    /// Imposta la quota del periodo. Sopra il tetto è rifiutata e la quota non cambia.
    pub fn set_quota(&mut self, task: usize, units: u32) -> Result<(), Error> {
        if units > CEILING {
            return Err(Error::OverCeiling);
        }
        self.slot(task)?.quota = units;
        Ok(())
    }

    /// Aggiunge uso al task (satura, non va in overflow).
    pub fn charge(&mut self, task: usize, units: u32) -> Result<(), Error> {
        let s = self.slot(task)?;
        s.used = s.used.saturating_add(units);
        Ok(())
    }

    /// Unità ancora disponibili nel periodo.
    pub fn remaining(&self, task: usize) -> Result<u32, Error> {
        match self.slots.get(task) {
            Some(s) if s.active => Ok(s.quota.saturating_sub(s.used)),
            _ => Err(Error::NoSuchTask),
        }
    }

    /// Prossimo task con quota residua, in ordine round-robin. `None` se tutti sono esauriti.
    /// Non addebita nulla: l'uso va registrato con `charge`.
    pub fn pick_next(&mut self) -> Option<usize> {
        for k in 0..N {
            let i = (self.next + k) % N;
            let s = &self.slots[i];
            if s.active && s.used < s.quota {
                self.next = (i + 1) % N;
                return Some(i);
            }
        }
        None
    }

    /// Nuovo periodo: azzera tutti i contatori di uso.
    pub fn refill(&mut self) {
        for s in self.slots.iter_mut() {
            s.used = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_robin_is_fair() {
        let mut q = Quota::<3>::new();
        let (a, b, c) = (q.add().unwrap(), q.add().unwrap(), q.add().unwrap());
        for t in [a, b, c] {
            q.set_quota(t, 10).unwrap();
        }
        let mut order = [0usize; 6];
        for slot in order.iter_mut() {
            let t = q.pick_next().unwrap();
            q.charge(t, 1).unwrap();
            *slot = t;
        }
        assert_eq!(order, [a, b, c, a, b, c]);
        assert_eq!(q.remaining(a), Ok(8));
    }

    #[test]
    fn exhausted_tasks_are_skipped() {
        let mut q = Quota::<4>::new();
        let (a, b, c) = (q.add().unwrap(), q.add().unwrap(), q.add().unwrap());
        q.set_quota(a, 1).unwrap();
        q.set_quota(b, 5).unwrap();
        q.set_quota(c, 5).unwrap();
        q.charge(a, 1).unwrap();
        assert_eq!(q.remaining(a), Ok(0));
        assert_eq!(q.pick_next(), Some(b));
        assert_eq!(q.pick_next(), Some(c));
        assert_eq!(q.pick_next(), Some(b));
        // Un task senza quota (quota 0) non viene mai scelto.
        let d = q.add().unwrap();
        assert_ne!(q.pick_next(), Some(d));
    }

    #[test]
    fn none_when_everyone_exhausted() {
        let mut q = Quota::<2>::new();
        assert_eq!(q.pick_next(), None);
        let a = q.add().unwrap();
        q.set_quota(a, 2).unwrap();
        q.charge(a, 5).unwrap(); // satura, non va in overflow
        assert_eq!(q.remaining(a), Ok(0));
        assert_eq!(q.pick_next(), None);
    }

    #[test]
    fn over_ceiling_quota_rejected() {
        let mut q = Quota::<1>::new();
        let a = q.add().unwrap();
        q.set_quota(a, CEILING).unwrap();
        assert_eq!(q.set_quota(a, CEILING + 1), Err(Error::OverCeiling));
        assert_eq!(q.set_quota(a, u32::MAX), Err(Error::OverCeiling));
        assert_eq!(q.remaining(a), Ok(CEILING)); // quota invariata
        assert_eq!(q.set_quota(7, 1), Err(Error::NoSuchTask));
    }

    #[test]
    fn refill_restores() {
        let mut q = Quota::<2>::new();
        let a = q.add().unwrap();
        q.set_quota(a, 3).unwrap();
        q.charge(a, 3).unwrap();
        assert_eq!(q.pick_next(), None);
        q.refill();
        assert_eq!(q.remaining(a), Ok(3));
        assert_eq!(q.pick_next(), Some(a));
    }

    #[test]
    fn table_full() {
        let mut q = Quota::<1>::new();
        assert!(q.add().is_ok());
        assert_eq!(q.add(), Err(Error::Full));
    }
}
