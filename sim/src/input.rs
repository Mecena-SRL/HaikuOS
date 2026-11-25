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

pub struct Input {
    queue: Queue<Event>,
    dropped: u32,
}

impl Input {
    pub fn new(depth: usize) -> Self {
        Self { queue: Queue::new(depth), dropped: 0 }
    }

    /// Errore `Full` se la coda è piena: l'evento si scarta e si conta.
    pub fn push(&mut self, e: Event) -> Result<(), Error> {
        let r = self.queue.push(e);
        if r.is_err() {
            self.dropped += 1;
        }
        r
    }

    pub fn poll(&mut self) -> Option<Event> {
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
