//! Microkernel input nel simulatore: coda fissa di eventi, senza allocazione dopo `new`.
//! Un evento in più sulla coda piena viene scartato e contato. Nessun hardware reale.
//! Regole: docs/resource-microkernels.md.

use crate::resource::{Arch, Error, Kind, Manifest, Microkernel, Queue};

/// Manifest generico: pulsanti e polling, su tutte le architetture.
pub const INPUT_MANIFEST: Manifest = Manifest {
    name: "input-generic",
    kind: Kind::Input,
    archs: &[Arch::Gba, Arch::Arm64, Arch::X86_64],
    required: true,
    bytes: 1024,
    queue_depth: 16,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    A,
    B,
    Select,
    Start,
    Right,
    Left,
    Up,
    Down,
    R,
    L,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Press(Button),
    Release(Button),
}

/// Evento con l'istante di arrivo (unità scelta dal chiamante, es. ms di sistema).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamped {
    pub at: u64,
    pub ev: Event,
}

pub struct Input {
    queue: Queue<Stamped>,
    dropped: u32,
}

impl Input {
    pub fn new(depth: usize) -> Self {
        Self { queue: Queue::new(depth), dropped: 0 }
    }

    /// Errore `Full` se la coda è piena: l'evento si scarta e si conta. Istante 0.
    pub fn push(&mut self, e: Event) -> Result<(), Error> {
        self.push_at(0, e)
    }

    /// Come `push`, con l'istante di arrivo.
    pub fn push_at(&mut self, at: u64, e: Event) -> Result<(), Error> {
        let r = self.queue.push(Stamped { at, ev: e });
        if r.is_err() {
            self.dropped += 1;
        }
        r
    }

    pub fn poll(&mut self) -> Option<Event> {
        self.queue.pop().map(|s| s.ev)
    }

    /// Come `poll`, ma con l'istante di arrivo.
    pub fn poll_stamped(&mut self) -> Option<Stamped> {
        self.queue.pop()
    }

    pub fn dropped(&self) -> u32 {
        self.dropped
    }
}

impl Microkernel for Input {
    fn manifest(&self) -> Manifest {
        INPUT_MANIFEST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn istanti_in_ordine_fifo() {
        let mut i = Input::new(4);
        i.push_at(10, Event::Press(Button::A)).unwrap();
        i.push_at(25, Event::Release(Button::A)).unwrap();
        let a = i.poll_stamped().unwrap();
        assert_eq!((a.at, a.ev), (10, Event::Press(Button::A)));
        assert_eq!(i.poll(), Some(Event::Release(Button::A)));
        assert_eq!(i.poll_stamped(), None);
    }

    #[test]
    fn push_senza_istante_vale_zero() {
        let mut i = Input::new(2);
        i.push(Event::Press(Button::B)).unwrap();
        assert_eq!(i.poll_stamped().unwrap().at, 0);
    }
}
