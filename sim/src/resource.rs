//! Contratto comune dei microkernel per risorsa, nel simulatore host.
//! Nessun hardware reale. Regole in docs/resource-microkernels.md.
//! Ogni driver ha la propria coda fissa; i driver avanzano in modo indipendente.
//! Nessun heap: il contratto usa solo `core`, compilabile senza `std`.

use core::ops::{Deref, DerefMut};

/// Capacità massima di una coda, se non specificata. Ogni coda occupa questo spazio fisso.
pub const QUEUE_MAX: usize = 32;
/// Microkernel attivabili insieme, e nomi in un piano.
pub const MAX_KERNELS: usize = 16;

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
    Network,
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

/// Coda FIFO a capacità fissa `N` (in uso `depth <= N`). Piena → `Error::Full`.
/// Nessuna allocazione: lo spazio è un array nel tipo.
#[derive(Debug, PartialEq, Eq)]
pub struct Queue<T, const N: usize = QUEUE_MAX> {
    items: [Option<T>; N],
    head: usize,
    len: usize,
    depth: usize,
}

impl<T, const N: usize> Queue<T, N> {
    /// Panics se `depth > N`: la capacità è decisa a compilazione.
    pub fn new(depth: usize) -> Self {
        assert!(depth <= N, "coda: profondità {depth} oltre la capacità {N}");
        Self { items: core::array::from_fn(|_| None), head: 0, len: 0, depth }
    }

    pub fn push(&mut self, t: T) -> Result<(), Error> {
        if self.len >= self.depth {
            return Err(Error::Full);
        }
        let at = (self.head + self.len) % N;
        self.items[at] = Some(t);
        self.len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let t = self.items[self.head].take();
        self.head = (self.head + 1) % N;
        self.len -= 1;
        t
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// Posti liberi.
    pub fn free(&self) -> usize {
        self.depth - self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Svuota la coda restituendo gli elementi in ordine.
    pub fn drain_all(&mut self) -> impl Iterator<Item = T> + '_ {
        core::iter::from_fn(move || self.pop())
    }
}

/// Stato comune di un microkernel: pronto o fermato.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ready,
    Stopped,
}

/// Contratto comune: ogni microkernel dichiara il proprio manifest e il proprio stato.
/// Lo stato di default è `Ready`; chi può fermarsi lo sovrascrive.
pub trait Microkernel {
    fn manifest(&self) -> Manifest;
    fn status(&self) -> Status {
        Status::Ready
    }
}

const IDLE: Manifest = Manifest {
    name: "",
    kind: Kind::Cpu,
    archs: &[],
    required: false,
    bytes: 0,
    queue_depth: 0,
};

#[derive(Debug, PartialEq, Eq)]
pub struct Driver {
    manifest: Manifest,
    queue: Queue<Request>,
    /// Completamenti non ancora letti. Invariante: `queue.len() + done.len() <= QUEUE_MAX`.
    done: Queue<Completion>,
    running: bool,
}

impl Driver {
    pub fn new(manifest: Manifest) -> Self {
        let queue = Queue::new(manifest.queue_depth);
        Self { manifest, queue, done: Queue::new(QUEUE_MAX), running: true }
    }

    /// Segnaposto senza lavoro, per riempire `Drivers`.
    fn idle() -> Self {
        Self { manifest: IDLE, queue: Queue::new(0), done: Queue::new(0), running: false }
    }

    pub fn name(&self) -> &'static str {
        self.manifest.name
    }

    /// Richiesta asincrona: accettata in coda, completata più tardi da `step`.
    pub fn submit(&mut self, r: Request) -> Result<(), Error> {
        if !self.running {
            return Err(Error::Stopped);
        }
        // Richieste in coda più completamenti non letti non superano la capacità.
        if self.queue.len() + self.done.len() >= QUEUE_MAX {
            return Err(Error::Full);
        }
        self.queue.push(r)
    }

    /// Esegue una richiesta in coda. Lavoro simulato: `op + 1`. Ritorna true se ne ha eseguita una.
    pub fn step(&mut self) -> bool {
        match self.queue.pop() {
            Some(r) => {
                let _ = self.done.push(Completion { id: r.id, result: Ok(r.op.wrapping_add(1)) });
                true
            }
            None => false,
        }
    }

    pub fn take_completions(&mut self) -> impl Iterator<Item = Completion> + '_ {
        self.done.drain_all()
    }

    /// Ferma solo questo driver: le richieste in coda falliscono, gli altri non cambiano.
    pub fn stop(&mut self) {
        self.running = false;
        while let Some(r) = self.queue.pop() {
            let _ = self.done.push(Completion { id: r.id, result: Err(Error::Stopped) });
        }
    }
}

impl Microkernel for Driver {
    fn manifest(&self) -> Manifest {
        self.manifest.clone()
    }
    fn status(&self) -> Status {
        if self.running { Status::Ready } else { Status::Stopped }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PlanError {
    UnknownArch,
    RequiredIncompatible(&'static str),
    NotEnoughRam { need: usize, have: usize },
    /// Più microkernel di `MAX_KERNELS`.
    TooMany,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub active: Names,
    /// Componenti opzionali incompatibili: saltati, da registrare nel log.
    pub skipped: Names,
    pub bytes: usize,
}

/// Elenco di nomi a capacità fissa (`MAX_KERNELS`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Names {
    buf: [&'static str; MAX_KERNELS],
    len: usize,
}

impl Names {
    pub fn new() -> Self {
        Self { buf: [""; MAX_KERNELS], len: 0 }
    }

    /// Chi la chiama garantisce `len < MAX_KERNELS`.
    fn push(&mut self, name: &'static str) {
        self.buf[self.len] = name;
        self.len += 1;
    }

    pub fn as_slice(&self) -> &[&'static str] {
        &self.buf[..self.len]
    }
}

/// Rilevamento e verifica: nessuna modifica di stato.
pub fn plan(target: Target, manifests: &[Manifest]) -> Result<Plan, PlanError> {
    let arch = target.arch.ok_or(PlanError::UnknownArch)?;
    if manifests.len() > MAX_KERNELS {
        return Err(PlanError::TooMany);
    }
    let mut active = Names::new();
    let mut skipped = Names::new();
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
    pub rolled_back: Names,
}

/// Driver attivi, a capacità fissa. Si usano come slice: `ds[0]`, `&mut ds`.
#[derive(Debug, PartialEq, Eq)]
pub struct Drivers {
    items: [Driver; MAX_KERNELS],
    len: usize,
}

impl Deref for Drivers {
    type Target = [Driver];
    fn deref(&self) -> &[Driver] {
        &self.items[..self.len]
    }
}

impl DerefMut for Drivers {
    fn deref_mut(&mut self) -> &mut [Driver] {
        &mut self.items[..self.len]
    }
}

/// Attivazione in ordine di dipendenza. Se una fallisce (o non c'è più posto), ferma quelle già
/// attive in ordine inverso e non restituisce nulla: nessuno stato parziale.
pub fn activate(
    manifests: &[Manifest],
    fail: impl Fn(&Manifest) -> bool,
) -> Result<Drivers, ActivateError> {
    let mut out = Drivers { items: core::array::from_fn(|_| Driver::idle()), len: 0 };
    for m in manifests {
        if fail(m) || out.len == MAX_KERNELS {
            let mut rolled_back = Names::new();
            while out.len > 0 {
                out.len -= 1;
                let d = &mut out.items[out.len];
                d.stop();
                rolled_back.push(d.name());
            }
            return Err(ActivateError { failed: m.name, rolled_back });
        }
        out.items[out.len] = Driver::new(m.clone());
        out.len += 1;
    }
    Ok(out)
}

/// Esegue un passo per driver. I driver sono indipendenti: un passo di uno non blocca gli altri.
pub fn tick(drivers: &mut [Driver]) -> usize {
    drivers.iter_mut().map(|d| d.step() as usize).sum()
}

/// Il monolite legge lo stato e sceglie: primo microkernel `Ready` del tipo richiesto.
/// Con più GPU, se una è ferma il lavoro va alla successiva.
pub fn pick_ready(ks: &[&dyn Microkernel], kind: Kind) -> Option<usize> {
    ks.iter()
        .position(|k| k.status() == Status::Ready && k.manifest().kind == kind)
}
