//! Microkernel input nel simulatore: coda fissa di eventi, senza allocazione dopo `new`.
//! Un evento in più sulla coda piena viene scartato e contato. Nessun hardware reale.
//! Regole: docs/resource-microkernels.md.

use crate::resource::{Arch, Error, Kind, Manifest};
use std::collections::VecDeque;

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
    queue: VecDeque<Event>,
    depth: usize,
    dropped: u32,
}

impl Input {
    pub fn new(depth: usize) -> Self {
        Self { queue: VecDeque::with_capacity(depth), depth, dropped: 0 }
    }

    /// Errore `Full` se la coda è piena: l'evento si scarta e si conta.
    pub fn push(&mut self, e: Event) -> Result<(), Error> {
        if self.queue.len() >= self.depth {
            self.dropped += 1;
            return Err(Error::Full);
        }
        self.queue.push_back(e);
        Ok(())
    }

    pub fn poll(&mut self) -> Option<Event> {
        self.queue.pop_front()
    }

    pub fn dropped(&self) -> u32 {
        self.dropped
    }
}
