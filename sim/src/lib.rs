//! Simulazione sull'host del monolite orizzontale e dell'agente di allocazione.
//! Nessun hardware, nessun bootloader.
//! Scelta: `std` (Vec) solo per la simulazione; la logica è piccola e portabile a `no_std` + `alloc`.

pub mod agent;
pub mod gpu;
pub mod cpu;
pub mod install;
pub mod lzss;
pub mod net;
pub mod memory;
pub mod process;
pub mod resource;
pub mod slot;
pub mod snapshot;
pub mod storage;
pub use agent::{Agent, GreedyAgent, Proposal, QuotaRequest};

// Limiti di partenza: docs/architecture.md, sezione 6. Tutti da verificare.
pub const RAM_BYTES: usize = 288 * 1024;
pub const MONOLITH_BYTES: usize = 64 * 1024;
pub const AGENT_BYTES: usize = 16 * 1024;
pub const SLOT_BYTES: usize = 32 * 1024;
pub const MAX_PROGRAMS: usize = 4;
/// Slot sempre libero. Lettura scelta: 4 slot, al massimo 3 concessi insieme. Da verificare.
pub const RESERVED_FREE_SLOTS: usize = 1;
pub const MAX_GRANTS: usize = MAX_PROGRAMS - RESERVED_FREE_SLOTS;
pub const POOL_BYTES: usize = RAM_BYTES - MONOLITH_BYTES - AGENT_BYTES;
/// Dimensione fissa massima di un messaggio. Da verificare.
pub const MAX_MESSAGE_BYTES: usize = 64;

// Controllo a compile time: i limiti stanno nella RAM.
const _: () = assert!(MONOLITH_BYTES + AGENT_BYTES + MAX_GRANTS * SLOT_BYTES <= RAM_BYTES);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgramId(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenyReason {
    UnknownProgram,
    AlreadyGranted,
    TooLarge,
    NoFreeSlot,
    /// L'agente ha proposto una quota fuori dai limiti o più grande di quella richiesta.
    AgentOverreach,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Granted(usize),
    Denied(DenyReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterError {
    NotAllowed,
    TooLarge,
    NoSlot,
}

pub struct Monolith<A: Agent> {
    agent: A,
    programs: Vec<ProgramId>,
    grants: Vec<(ProgramId, usize)>,
    rules: Vec<(ProgramId, ProgramId)>,
    inbox: Vec<(ProgramId, Vec<u8>)>,
}

impl<A: Agent> Monolith<A> {
    pub fn new(agent: A) -> Self {
        Self { agent, programs: Vec::new(), grants: Vec::new(), rules: Vec::new(), inbox: Vec::new() }
    }

    /// Registra un programma. Massimo `MAX_PROGRAMS`.
    pub fn register(&mut self, p: ProgramId) -> bool {
        if self.programs.len() >= MAX_PROGRAMS || self.programs.contains(&p) {
            return false;
        }
        self.programs.push(p);
        true
    }

    /// Regola fissa mittente -> destinatario. Solo il monolite la aggiunge.
    pub fn allow(&mut self, from: ProgramId, to: ProgramId) {
        self.rules.push((from, to));
    }

    /// Richiesta di quota. L'agente propone, il monolite verifica tutto.
    pub fn request(&mut self, p: ProgramId, bytes: usize) -> Decision {
        if !self.programs.contains(&p) {
            return Decision::Denied(DenyReason::UnknownProgram);
        }
        if self.grant_of(p).is_some() {
            return Decision::Denied(DenyReason::AlreadyGranted);
        }
        if bytes == 0 || bytes > SLOT_BYTES {
            return Decision::Denied(DenyReason::TooLarge);
        }
        // Lo slot riservato non è mai concesso: si nega senza nemmeno interpellare l'agente.
        if self.grants.len() >= MAX_GRANTS {
            return Decision::Denied(DenyReason::NoFreeSlot);
        }
        let req = QuotaRequest { program: p, bytes };
        let free = MAX_GRANTS - self.grants.len();
        let granted = match self.agent.propose(&req, free) {
            Proposal::Deny(r) => return Decision::Denied(r),
            Proposal::Grant(b) => b,
        };
        // Non ci si fida dell'agente: quota > 0, non oltre la richiesta né lo slot, nel pool.
        let used: usize = self.grants.iter().map(|(_, b)| b).sum();
        let ok = granted >= 1
            && granted <= bytes
            && used + granted <= POOL_BYTES
            && self.grants.len() < MAX_GRANTS;
        if !ok {
            return Decision::Denied(DenyReason::AgentOverreach);
        }
        self.grants.push((p, granted));
        Decision::Granted(granted)
    }

    /// Revoca la quota. Libera lo slot.
    pub fn revoke(&mut self, p: ProgramId) -> bool {
        let before = self.grants.len();
        self.grants.retain(|(g, _)| *g != p);
        self.grants.len() != before
    }

    pub fn grant_of(&self, p: ProgramId) -> Option<usize> {
        self.grants.iter().find(|(g, _)| *g == p).map(|(_, b)| *b)
    }

    pub fn granted_count(&self) -> usize {
        self.grants.len()
    }

    pub fn inbox(&self) -> &[(ProgramId, Vec<u8>)] {
        &self.inbox
    }

    /// Filtro messaggi: allowlist, dimensione fissa, solo tra programmi con slot.
    /// Il contenuto non concede né modifica diritti.
    pub fn send(&mut self, from: ProgramId, to: ProgramId, payload: &[u8]) -> Result<(), FilterError> {
        if !self.rules.contains(&(from, to)) {
            return Err(FilterError::NotAllowed);
        }
        if payload.len() > MAX_MESSAGE_BYTES {
            return Err(FilterError::TooLarge);
        }
        if self.grant_of(from).is_none() || self.grant_of(to).is_none() {
            return Err(FilterError::NoSlot);
        }
        self.inbox.push((to, payload.to_vec()));
        Ok(())
    }
}
