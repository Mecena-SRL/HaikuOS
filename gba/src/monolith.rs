//! Monolite e agente di allocazione, porting no_std di sim/src/lib.rs.
//! Array fissi, nessun heap. L'agente propone; il monolite verifica.

pub const MAX_PROGRAMS: usize = 4;
/// Uno slot sempre libero: al massimo 3 quote insieme. Da verificare.
pub const MAX_GRANTS: usize = MAX_PROGRAMS - 1;
pub const SLOT_BYTES: usize = 32 * 1024;
pub const MONOLITH_BYTES: usize = 64 * 1024;
pub const AGENT_BYTES: usize = 16 * 1024;
/// EWRAM 256 KB + IWRAM 32 KB. Da verificare: quanto resta davvero libero.
pub const RAM_BYTES: usize = 288 * 1024;
pub const POOL_BYTES: usize = RAM_BYTES - MONOLITH_BYTES - AGENT_BYTES;

const _: () = assert!(MONOLITH_BYTES + AGENT_BYTES + MAX_GRANTS * SLOT_BYTES <= RAM_BYTES);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Deny {
    UnknownProgram,
    AlreadyGranted,
    TooLarge,
    NoFreeSlot,
    AgentOverreach,
}

pub struct Monolith {
    registered: [bool; MAX_PROGRAMS],
    /// Byte concessi per programma; 0 = nessuna quota.
    grants: [usize; MAX_PROGRAMS],
}

impl Monolith {
    pub const fn new() -> Self {
        Self { registered: [false; MAX_PROGRAMS], grants: [0; MAX_PROGRAMS] }
    }

    pub fn register(&mut self, p: usize) {
        if p < MAX_PROGRAMS {
            self.registered[p] = true;
        }
    }

    pub fn granted_count(&self) -> usize {
        self.grants.iter().filter(|&&b| b != 0).count()
    }

    pub fn grant_of(&self, p: usize) -> usize {
        self.grants.get(p).copied().unwrap_or(0)
    }

    /// Richiesta di quota. Agente di riferimento: concede la richiesta se c'è uno slot.
    pub fn request(&mut self, p: usize, bytes: usize) -> Result<usize, Deny> {
        if p >= MAX_PROGRAMS || !self.registered[p] {
            return Err(Deny::UnknownProgram);
        }
        if self.grants[p] != 0 {
            return Err(Deny::AlreadyGranted);
        }
        if bytes == 0 || bytes > SLOT_BYTES {
            return Err(Deny::TooLarge);
        }
        // Lo slot riservato non si concede mai.
        if self.granted_count() >= MAX_GRANTS {
            return Err(Deny::NoFreeSlot);
        }
        // Proposta dell'agente, poi verifica del monolite (non ci si fida dell'agente).
        let granted = bytes;
        let used: usize = self.grants.iter().sum();
        if granted == 0 || granted > bytes || used + granted > POOL_BYTES {
            return Err(Deny::AgentOverreach);
        }
        self.grants[p] = granted;
        Ok(granted)
    }

    pub fn revoke(&mut self, p: usize) {
        if p < MAX_PROGRAMS {
            self.grants[p] = 0;
        }
    }
}
