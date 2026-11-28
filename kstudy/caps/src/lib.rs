//! Capacità per programma in stile pledge/unveil (idea 15 di docs/kernel-study.md).
//! L'insieme di operazioni può solo restringersi; i percorsi visibili si fissano con `lock()`.
//! Nessun I/O, nessuna allocazione: tabella a capacità costante `N`.
#![cfg_attr(not(test), no_std)]

pub const OP_READ: u32 = 1 << 0;
pub const OP_WRITE: u32 = 1 << 1;
pub const OP_EXEC: u32 = 1 << 2;
pub const OP_NET: u32 = 1 << 3;
pub const OP_SPAWN: u32 = 1 << 4;
pub const OP_DEVICE: u32 = 1 << 5;
pub const OP_CLOCK: u32 = 1 << 6;
pub const OP_IPC: u32 = 1 << 7;
pub const OP_ALL: u32 = 0xff;

pub const PERM_READ: u8 = 1 << 0;
pub const PERM_WRITE: u8 = 1 << 1;

/// Operazioni o permessi non concessi. Contiene i bit richiesti che mancano.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Denied(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnveilError {
    /// `lock()` già chiamato: la tabella è fissa.
    Locked,
    /// Tabella piena (N voci).
    Full,
    /// Il prefisso deve essere assoluto (inizia con `/`).
    BadPrefix,
}

#[derive(Clone, Copy)]
struct Entry {
    prefix: &'static str,
    perms: u8,
}

pub struct Caps<const N: usize> {
    ops: u32,
    table: [Option<Entry>; N],
    locked: bool,
}

impl<const N: usize> Caps<N> {
    pub const fn new(ops: u32) -> Self {
        Caps { ops, table: [None; N], locked: false }
    }

    pub fn ops(&self) -> u32 {
        self.ops
    }

    /// Toglie bit dall'insieme. È l'unico modo di cambiarlo: non si possono aggiungere bit.
    pub fn restrict(&mut self, mask: u32) {
        self.ops &= mask;
    }

    /// Controlla un insieme di operazioni: tutti i bit richiesti devono essere concessi.
    pub fn check(&self, op: u32) -> Result<(), Denied> {
        match op & !self.ops {
            0 => Ok(()),
            missing => Err(Denied(missing)),
        }
    }

    /// Aggiunge o sostituisce i permessi di un prefisso. Possibile solo prima di `lock()`.
    pub fn unveil(&mut self, prefix: &'static str, perms: u8) -> Result<(), UnveilError> {
        if self.locked {
            return Err(UnveilError::Locked);
        }
        let p = normalize(prefix).ok_or(UnveilError::BadPrefix)?;
        let idx = self
            .table
            .iter()
            .position(|s| matches!(s, Some(e) if e.prefix == p))
            .or_else(|| self.table.iter().position(|s| s.is_none()))
            .ok_or(UnveilError::Full)?;
        self.table[idx] = Some(Entry { prefix: p, perms });
        Ok(())
    }

    /// Fissa la tabella: da qui in poi `unveil` fallisce sempre.
    pub fn lock(&mut self) {
        self.locked = true;
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Controlla un percorso con il prefisso più lungo che lo copre.
    /// Un `..` come componente viene sempre negato.
    pub fn check_path(&self, path: &str, want: u8) -> Result<(), Denied> {
        if path.split('/').any(|c| c == "..") {
            return Err(Denied(want as u32));
        }
        let best = self
            .table
            .iter()
            .flatten()
            .filter(|e| covers(e.prefix, path))
            .max_by_key(|e| e.prefix.len());
        let have = best.map_or(0, |e| e.perms);
        match want & !have {
            0 => Ok(()),
            missing => Err(Denied(missing as u32)),
        }
    }
}

/// Toglie le `/` finali; "/" resta "/". Rifiuta i percorsi non assoluti.
fn normalize(p: &'static str) -> Option<&'static str> {
    if !p.starts_with('/') {
        return None;
    }
    let t = p.trim_end_matches('/');
    Some(if t.is_empty() { "/" } else { t })
}

/// Vero se `prefix` copre `path` su un confine di componente: "/data" non copre "/database".
fn covers(prefix: &str, path: &str) -> bool {
    if prefix == "/" {
        return path.starts_with('/');
    }
    path.strip_prefix(prefix)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restrict_only_shrinks() {
        let mut c = Caps::<4>::new(OP_READ | OP_WRITE);
        c.restrict(OP_READ | OP_NET);
        assert_eq!(c.ops(), OP_READ);
        // Nessun modo di riaggiungere: né OP_NET né OP_WRITE tornano.
        c.restrict(OP_ALL);
        assert_eq!(c.ops(), OP_READ);
        assert_eq!(c.check(OP_WRITE), Err(Denied(OP_WRITE)));
        assert_eq!(c.check(OP_NET), Err(Denied(OP_NET)));
    }

    #[test]
    fn denied_op_reports_missing_bits() {
        let c = Caps::<1>::new(OP_READ | OP_EXEC);
        assert_eq!(c.check(OP_READ), Ok(()));
        assert_eq!(c.check(OP_READ | OP_EXEC), Ok(()));
        assert_eq!(c.check(OP_READ | OP_SPAWN | OP_NET), Err(Denied(OP_SPAWN | OP_NET)));
        assert_eq!(c.check(0), Ok(()));
    }

    #[test]
    fn unveil_before_lock_then_fixed() {
        let mut c = Caps::<2>::new(OP_READ);
        assert_eq!(c.unveil("/etc", PERM_READ), Ok(()));
        assert_eq!(c.check_path("/etc/hosts", PERM_READ), Ok(()));
        assert_eq!(c.check_path("/etc/hosts", PERM_WRITE), Err(Denied(PERM_WRITE as u32)));
        c.lock();
        assert!(c.is_locked());
        assert_eq!(c.unveil("/tmp", PERM_READ | PERM_WRITE), Err(UnveilError::Locked));
        assert_eq!(c.unveil("/etc", PERM_READ | PERM_WRITE), Err(UnveilError::Locked));
        // La tabella non è cambiata dopo il lock.
        assert_eq!(c.check_path("/tmp/x", PERM_READ), Err(Denied(PERM_READ as u32)));
        assert_eq!(c.check_path("/etc/x", PERM_WRITE), Err(Denied(PERM_WRITE as u32)));
    }

    #[test]
    fn unveil_table_full_and_bad_prefix() {
        let mut c = Caps::<1>::new(0);
        assert_eq!(c.unveil("data", PERM_READ), Err(UnveilError::BadPrefix));
        assert_eq!(c.unveil("/a", PERM_READ), Ok(()));
        assert_eq!(c.unveil("/a", PERM_WRITE), Ok(())); // stesso prefisso: sostituisce
        assert_eq!(c.unveil("/b", PERM_READ), Err(UnveilError::Full));
        assert_eq!(c.check_path("/a/f", PERM_WRITE), Ok(()));
    }

    #[test]
    fn component_boundary() {
        let mut c = Caps::<2>::new(0);
        c.unveil("/data", PERM_READ).unwrap();
        assert_eq!(c.check_path("/data", PERM_READ), Ok(()));
        assert_eq!(c.check_path("/data/f.txt", PERM_READ), Ok(()));
        assert_eq!(c.check_path("/database", PERM_READ), Err(Denied(PERM_READ as u32)));
        assert_eq!(c.check_path("/databases/x", PERM_READ), Err(Denied(PERM_READ as u32)));
        // Prefisso con "/" finale: stessa regola.
        c.unveil("/logs/", PERM_READ).unwrap();
        assert_eq!(c.check_path("/logs/a", PERM_READ), Ok(()));
        assert_eq!(c.check_path("/logsx", PERM_READ), Err(Denied(PERM_READ as u32)));
    }

    #[test]
    fn longest_prefix_wins_and_dotdot_denied() {
        let mut c = Caps::<3>::new(0);
        c.unveil("/", PERM_READ | PERM_WRITE).unwrap();
        c.unveil("/usr", PERM_READ).unwrap();
        assert_eq!(c.check_path("/usr/bin/sh", PERM_WRITE), Err(Denied(PERM_WRITE as u32)));
        assert_eq!(c.check_path("/home/u", PERM_WRITE), Ok(()));
        assert_eq!(c.check_path("/usr/../etc/x", PERM_READ), Err(Denied(PERM_READ as u32)));
    }
}
