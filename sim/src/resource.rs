//! Contratto comune dei microkernel per risorsa, nel simulatore host.
//! Nessun hardware reale. Regole in docs/resource-microkernels.md.
//! Ogni driver ha la propria coda fissa; i driver avanzano in modo indipendente.

use std::collections::VecDeque;

/// Architetture note. Il 128 bit non ha CPU general purpose in uso: non c'è variante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arch {
    /// ARM7TDMI, 32 bit (GBA).
    Gba,
    /// ARM64 (include Apple Silicon: differenze come feature, da verificare).
    Arm64,
    /// x86-64.
    X86_64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Cpu,
    Gpu,
    Memory,
    Storage,
    Swap,
    Audio,
    Input,
    Power,
    Crypto,
}

/// Dichiarazione di un microkernel: cosa è, dove gira, quanto costa.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub name: &'static str,
    pub kind: Kind,
    pub archs: &'static [Arch],
    pub required: bool,
    pub bytes: usize,
    pub queue_depth: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    /// `None` = architettura sconosciuta: nulla viene caricato (fail closed).
    pub arch: Option<Arch>,
    pub ram_free: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Full,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub id: u64,
    pub op: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Completion {
    pub id: u64,
    pub result: Result<u32, Error>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Driver {
    manifest: Manifest,
    pending: VecDeque<Request>,
    done: Vec<Completion>,
    running: bool,
}

impl Driver {
    pub fn new(manifest: Manifest) -> Self {
        Self { manifest, pending: VecDeque::new(), done: Vec::new(), running: true }
    }

    pub fn name(&self) -> &'static str {
        self.manifest.name
    }

    /// Richiesta asincrona: accettata in coda, completata più tardi da `step`.
    pub fn submit(&mut self, r: Request) -> Result<(), Error> {
        if !self.running {
            return Err(Error::Stopped);
        }
        if self.pending.len() >= self.manifest.queue_depth {
            return Err(Error::Full);
        }
        self.pending.push_back(r);
        Ok(())
    }

    /// Esegue una richiesta in coda. Lavoro simulato: `op + 1`. Ritorna true se ne ha eseguita una.
    pub fn step(&mut self) -> bool {
        match self.pending.pop_front() {
            Some(r) => {
                self.done.push(Completion { id: r.id, result: Ok(r.op.wrapping_add(1)) });
                true
            }
            None => false,
        }
    }

    pub fn take_completions(&mut self) -> Vec<Completion> {
        std::mem::take(&mut self.done)
    }

    /// Ferma solo questo driver: le richieste in coda falliscono, gli altri non cambiano.
    pub fn stop(&mut self) {
        self.running = false;
        for r in self.pending.drain(..) {
            self.done.push(Completion { id: r.id, result: Err(Error::Stopped) });
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PlanError {
    UnknownArch,
    RequiredIncompatible(&'static str),
    NotEnoughRam { need: usize, have: usize },
}

#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub active: Vec<&'static str>,
    /// Componenti opzionali incompatibili: saltati, da registrare nel log.
    pub skipped: Vec<&'static str>,
    pub bytes: usize,
}

/// Rilevamento e verifica: nessuna modifica di stato.
pub fn plan(target: Target, manifests: &[Manifest]) -> Result<Plan, PlanError> {
    let arch = target.arch.ok_or(PlanError::UnknownArch)?;
    let mut active = Vec::new();
    let mut skipped = Vec::new();
    let mut bytes = 0;
    for m in manifests {
        if m.archs.contains(&arch) {
            active.push(m.name);
            bytes += m.bytes;
        } else if m.required {
            return Err(PlanError::RequiredIncompatible(m.name));
        } else {
            skipped.push(m.name);
        }
    }
    if bytes > target.ram_free {
        return Err(PlanError::NotEnoughRam { need: bytes, have: target.ram_free });
    }
    Ok(Plan { active, skipped, bytes })
}

#[derive(Debug, PartialEq, Eq)]
pub struct ActivateError {
    pub failed: &'static str,
    /// Driver già attivati e poi fermati, in ordine inverso di attivazione.
    pub rolled_back: Vec<&'static str>,
}

/// Attivazione in ordine di dipendenza. Se una fallisce, ferma quelle già attive
/// in ordine inverso e non restituisce nulla: nessuno stato parziale.
pub fn activate(
    manifests: &[Manifest],
    fail: impl Fn(&Manifest) -> bool,
) -> Result<Vec<Driver>, ActivateError> {
    let mut drivers: Vec<Driver> = Vec::new();
    for m in manifests {
        if fail(m) {
            let mut rolled_back = Vec::new();
            while let Some(mut d) = drivers.pop() {
                d.stop();
                rolled_back.push(d.name());
            }
            return Err(ActivateError { failed: m.name, rolled_back });
        }
        drivers.push(Driver::new(m.clone()));
    }
    Ok(drivers)
}

/// Esegue un passo per driver. I driver sono indipendenti: un passo di uno non blocca gli altri.
pub fn tick(drivers: &mut [Driver]) -> usize {
    drivers.iter_mut().map(|d| d.step() as usize).sum()
}
