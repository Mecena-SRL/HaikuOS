//! Monolite e agente di allocazione, porting no_std di sim/src/lib.rs.
//! Array fissi, nessun heap. L'agente propone; il monolite verifica.

use crate::alloc::{Arena, BLK};

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Deny {
    UnknownProgram,
    AlreadyGranted,
    TooLarge,
    NoFreeSlot,
    AgentOverreach,
    OverQuota,
}

pub struct Monolith {
    registered: [bool; MAX_PROGRAMS],
    /// Byte concessi per programma; 0 = nessuna quota.
    grants: [usize; MAX_PROGRAMS],
    /// Memoria allocata dai programmi, dentro la loro quota.
    arenas: [Arena; MAX_PROGRAMS],
}

impl Monolith {
    pub const fn new() -> Self {
        Self { registered: [false; MAX_PROGRAMS], grants: [0; MAX_PROGRAMS], arenas: [Arena::new(); MAX_PROGRAMS] }
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

    /// Alloca memoria per il programma `p`, entro la sua quota. Ritorna l'offset nel suo slot.
    pub fn alloc(&mut self, p: usize, bytes: usize) -> Result<usize, Deny> {
        if p >= MAX_PROGRAMS || self.grants[p] == 0 {
            return Err(Deny::UnknownProgram);
        }
        let used = self.arenas[p].used_blocks() * BLK;
        if used + bytes.div_ceil(BLK).max(1) * BLK > self.grants[p] {
            return Err(Deny::OverQuota);
        }
        self.arenas[p].alloc(bytes).ok_or(Deny::OverQuota)
    }

    pub fn free(&mut self, p: usize, off: usize, bytes: usize) -> bool {
        p < MAX_PROGRAMS && self.arenas[p].free(off, bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_stays_inside_grant() {
        let mut m = Monolith::new();
        m.register(0);
        assert_eq!(m.alloc(0, 4096), Err(Deny::UnknownProgram)); // nessuna quota
        assert_eq!(m.request(0, 8192), Ok(8192));
        assert_eq!(m.alloc(0, 4096), Ok(0));
        assert_eq!(m.alloc(0, 4096), Ok(4096));
        assert_eq!(m.alloc(0, 64), Err(Deny::OverQuota)); // quota piena
        assert!(m.free(0, 0, 4096));
        assert_eq!(m.alloc(0, 4096), Ok(0));
    }
}
