//! Persistenza: journal con checksum su un supporto a blocchi (SRAM sul GBA, file sulla macchina potente).
//! Due regioni di log alternate e due slot di intestazione. Ogni salvataggio (`sync`) scrive solo
//! i file cambiati dall'ultimo, con un record per file: più modifiche allo stesso file in una
//! finestra diventano una sola scrittura. Quando il log è pieno, lo stato corrente viene riscritto
//! nella regione libera (compattazione), poi cambia la testa.
//! Ogni record: [tipo][owner][lun. nome][nome][lun. dati u16][dati][checksum FNV-1a u32].
//! Dopo la coda del log c'è sempre un byte 0, così il replay non legge record vecchi.
#![allow(dead_code)]
use crate::fs::{Fs, FsError, FILES, MAX_FILE, NAME_MAX};

/// Profilo dichiarato dal livello sottile: quando salvare.
/// Salva quando si accumulano `units` modifiche, oppure dopo `max_frames` frame se c'è almeno una modifica.
/// Profilo macchina potente (file su disco): soglia 4 modifiche o 6 frame (0,1 s), nessun limite di salvataggi.
/// Valori scelti senza misure: da verificare.
pub const POWERFUL: Profile = Profile { units: 4, max_frames: 6, max_writes_per_min: u32::MAX };

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile {
    /// Soglia di modifiche (token) che fa partire un salvataggio.
    pub units: u32,
    /// Salvataggio a tempo, in frame (60 frame = 1 s sul GBA). 0 = solo a soglia.
    pub max_frames: u32,
    /// Limite di salvataggi per minuto (usura). `u32::MAX` = nessun limite.
    pub max_writes_per_min: u32,
}

/// Supporto a blocchi: il livello sottile per piattaforma.
pub trait Medium {
    fn read(&self, off: usize, out: &mut [u8]);
    fn write(&mut self, off: usize, data: &[u8]);
    fn profile(&self) -> Profile {
        Profile { units: 1, max_frames: 1, max_writes_per_min: u32::MAX }
    }
}

const SLOT: usize = 16;
const HDR_AREA: usize = 2 * SLOT;
const K_CREATE: u8 = 1;
const K_WRITE: u8 = 2;
const K_REMOVE: u8 = 3;
const K_MKDIR: u8 = 4;
const FNV0: u32 = 0x811c_9dc5;

fn upd(mut h: u32, b: &[u8]) -> u32 {
    for &x in b {
        h ^= x as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

fn rec_len(nl: usize, dl: usize) -> usize {
    3 + nl + 2 + dl + 4
}

/// Intestazione del record (tipo, owner, nome, lunghezza dati).
fn head(kind: u8, owner: u8, name: &[u8], dl: usize) -> ([u8; 3 + NAME_MAX + 2], usize) {
    let mut h = [0u8; 3 + NAME_MAX + 2];
    let hl = 3 + name.len() + 2;
    h[0] = kind;
    h[1] = owner;
    h[2] = name.len() as u8;
    h[3..hl - 2].copy_from_slice(name);
    h[hl - 2..hl].copy_from_slice(&(dl as u16).to_le_bytes());
    (h, hl)
}

/// Codifica un record piccolo nel lotto. Ritorna i byte scritti.
fn encode(kind: u8, owner: u8, name: &[u8], data: &[u8], out: &mut [u8]) -> usize {
    let (h, hl) = head(kind, owner, name, data.len());
    let total = rec_len(name.len(), data.len());
    let crc = upd(upd(FNV0, &h[..hl]), data).to_le_bytes();
    out[..hl].copy_from_slice(&h[..hl]);
    out[hl..hl + data.len()].copy_from_slice(data);
    out[total - 4..total].copy_from_slice(&crc);
    total
}

/// Scrive un record direttamente sul supporto. `off` avanza. Errore se non entra in `cap`.
fn put<M: Medium>(m: &mut M, base: usize, off: &mut usize, cap: usize, kind: u8, owner: u8, name: &[u8], data: &[u8]) -> Result<(), FsError> {
    let total = rec_len(name.len(), data.len());
    if *off + total > cap {
        return Err(FsError::NoSpace);
    }
    let (h, hl) = head(kind, owner, name, data.len());
    let at = base + *off;
    m.write(at, &h[..hl]);
    m.write(at + hl, data);
    m.write(at + hl + data.len(), &upd(upd(FNV0, &h[..hl]), data).to_le_bytes());
    *off += total;
    Ok(())
}

fn write_header<M: Medium>(m: &mut M, gen: u32, active: usize) {
    let mut b = [0u8; 11];
    b[0] = b'H';
    b[1] = b'K';
    b[2..6].copy_from_slice(&gen.to_le_bytes());
    b[6] = active as u8;
    let c = upd(FNV0, &b[..7]).to_le_bytes();
    b[7..11].copy_from_slice(&c);
    m.write((gen as usize & 1) * SLOT, &b);
}

fn read_header<M: Medium>(m: &M, slot: usize) -> Option<(u32, usize)> {
    let mut b = [0u8; 11];
    m.read(slot * SLOT, &mut b);
    if b[0] != b'H' || b[1] != b'K' || b[6] > 1 {
        return None;
    }
    if upd(FNV0, &b[..7]) != u32::from_le_bytes([b[7], b[8], b[9], b[10]]) {
        return None;
    }
    Some((u32::from_le_bytes([b[2], b[3], b[4], b[5]]), b[6] as usize))
}

/// Massimo di cancellazioni distinte tra due salvataggi, oltre il quale si compatta.
const RM_MAX: usize = 8;

pub struct Persist<M: Medium> {
    fs: Fs,
    m: M,
    region: usize,
    active: usize,
    gen: u32,
    end: usize,
    frame: u32,
    profile: Profile,
    /// File modificati dall'ultimo salvataggio (bit per id) e quelli creati di recente.
    dirty: u16,
    fresh: u16,
    removed: [([u8; NAME_MAX], u8, u8); RM_MAX],
    rm_len: usize,
    rm_overflow: bool,
    /// Salvataggi che hanno scritto qualcosa.
    pub saves: u32,
    /// Modifiche dall'ultimo salvataggio, frame dell'ultimo salvataggio, finestra di un minuto.
    pending: u32,
    last_save: u32,
    win_start: u32,
    win_saves: u32,
}

impl<M: Medium> Persist<M> {
    /// Apre il supporto: sceglie l'intestazione valida più recente e rigioca il log.
    /// Senza intestazioni valide, parte vuoto.
    pub fn open(m: M, region: usize) -> Self {
        let profile = m.profile();
        let mut p = Self {
            fs: Fs::new(), m, region, active: 0, gen: 0, end: 0, frame: 0, profile,
            dirty: 0, fresh: 0, removed: [([0; NAME_MAX], 0, 0); RM_MAX], rm_len: 0, rm_overflow: false, saves: 0,
            pending: 0, last_save: 0, win_start: 0, win_saves: 0,
        };
        let a = read_header(&p.m, 0);
        let b = read_header(&p.m, 1);
        let best = match (a, b) {
            (Some(x), Some(y)) => Some(if x.0 >= y.0 { x } else { y }),
            (x, y) => x.or(y),
        };
        match best {
            Some((gen, active)) => {
                p.gen = gen;
                p.active = active;
                p.end = p.replay();
            }
            None => {
                let _ = p.compact();
            }
        }
        p
    }

    fn replay(&mut self) -> usize {
        let base = HDR_AREA + self.active * self.region;
        let mut buf = [0u8; MAX_FILE];
        let mut off = 0;
        loop {
            let mut h = [0u8; 3];
            if off + 3 > self.region {
                break;
            }
            self.m.read(base + off, &mut h);
            let (kind, owner, nl) = (h[0], h[1], h[2] as usize);
            if !(K_CREATE..=K_MKDIR).contains(&kind) || nl == 0 || nl > NAME_MAX {
                break;
            }
            let hl = 3 + nl + 2;
            if off + hl > self.region {
                break;
            }
            let mut rest = [0u8; NAME_MAX + 2];
            self.m.read(base + off + 3, &mut rest[..nl + 2]);
            let dl = u16::from_le_bytes([rest[nl], rest[nl + 1]]) as usize;
            let total = rec_len(nl, dl);
            if dl > MAX_FILE || off + total > self.region {
                break;
            }
            self.m.read(base + off + hl, &mut buf[..dl]);
            let mut c = [0u8; 4];
            self.m.read(base + off + total - 4, &mut c);
            if upd(upd(upd(FNV0, &h), &rest[..nl + 2]), &buf[..dl]) != u32::from_le_bytes(c) {
                break;
            }
            self.apply(kind, owner, &rest[..nl], &buf[..dl]);
            off += total;
        }
        off
    }

    fn apply(&mut self, kind: u8, owner: u8, name: &[u8], data: &[u8]) {
        match kind {
            K_CREATE => {
                let _ = self.fs.create(name, owner);
            }
            K_WRITE => {
                if let Some(id) = self.fs.find_name(name) {
                    let o = self.fs.info(id).map_or(0, |x| x.1);
                    let _ = self.fs.write(id, o, MAX_FILE, data);
                }
            }
            K_MKDIR => {
                let _ = self.fs.mkdir(name, owner);
            }
            K_REMOVE => {
                if let Some(id) = self.fs.find_name(name) {
                    let o = self.fs.info(id).map_or(0, |x| x.1);
                    let _ = self.fs.remove(id, o);
                }
            }
            _ => {}
        }
    }

    /// Marca la fine del log con un byte 0.
    fn terminate(&mut self, base: usize, end: usize) {
        if end < self.region {
            self.m.write(base + end, &[0]);
        }
    }

    /// Salvataggio: scrive solo i file cambiati dall'ultimo. Ritorna Ok anche se ha compattato.
    pub fn sync(&mut self) -> Result<(), FsError> {
        if self.rm_overflow {
            return self.compact();
        }
        let dirty_any = self.dirty != 0 || self.rm_len > 0;
        if !dirty_any {
            return Ok(());
        }
        let base = HDR_AREA + self.active * self.region;
        let mut off = self.end;
        let mut buf = [0u8; MAX_FILE];
        let res = (|| -> Result<(), FsError> {
            for i in 0..self.rm_len {
                let (nm, nl, owner) = self.removed[i];
                put(&mut self.m, base, &mut off, self.region, K_REMOVE, owner, &nm[..nl as usize], &[])?;
            }
            for pass in 0..2 {
                for id in 0..FILES {
                    if self.dirty & (1 << id) == 0 || self.fs.is_dir(id) != (pass == 0) {
                        continue;
                    }
                    let (nm, nl, owner) = match self.fs.info(id) {
                        Some((n, o)) => {
                            let mut nm = [0u8; NAME_MAX];
                            nm[..n.len()].copy_from_slice(n);
                            (nm, n.len(), o)
                        }
                        None => continue,
                    };
                    let name = &nm[..nl];
                    if self.fresh & (1 << id) != 0 {
                        let kind = if pass == 0 { K_MKDIR } else { K_CREATE };
                        put(&mut self.m, base, &mut off, self.region, kind, owner, name, &[])?;
                    }
                    if pass == 1 {
                        let len = self.fs.len_of(id).unwrap_or(0);
                        self.fs.read(id, owner, &mut buf[..len])?;
                        if len > 0 {
                            put(&mut self.m, base, &mut off, self.region, K_WRITE, owner, name, &buf[..len])?;
                        }
                    }
                }
            }
            Ok(())
        })();
        match res {
            Ok(()) => {
                self.terminate(base, off);
                self.end = off;
                self.saved();
                self.dirty = 0;
                self.fresh = 0;
                self.rm_len = 0;
                Ok(())
            }
            Err(_) => self.compact(),
        }
    }

    /// Riscrive lo stato corrente nella regione libera, poi cambia la testa.
    fn compact(&mut self) -> Result<(), FsError> {
        let new = 1 - self.active;
        let base = HDR_AREA + new * self.region;
        let mut off = 0;
        let mut buf = [0u8; MAX_FILE];
        for pass in 0..2 {
            for id in 0..FILES {
                if self.fs.is_dir(id) != (pass == 0) {
                    continue;
                }
                let (nm, nl, owner) = match self.fs.info(id) {
                    Some((n, o)) => {
                        let mut nm = [0u8; NAME_MAX];
                        nm[..n.len()].copy_from_slice(n);
                        (nm, n.len(), o)
                    }
                    None => continue,
                };
                let name = &nm[..nl];
                if pass == 0 {
                    put(&mut self.m, base, &mut off, self.region, K_MKDIR, owner, name, &[])?;
                } else {
                    let len = self.fs.len_of(id).unwrap_or(0);
                    self.fs.read(id, owner, &mut buf[..len])?;
                    put(&mut self.m, base, &mut off, self.region, K_CREATE, owner, name, &[])?;
                    if len > 0 {
                        put(&mut self.m, base, &mut off, self.region, K_WRITE, owner, name, &buf[..len])?;
                    }
                }
            }
        }
        self.terminate(base, off);
        write_header(&mut self.m, self.gen + 1, new);
        self.gen += 1;
        self.active = new;
        self.end = off;
        self.dirty = 0;
        self.fresh = 0;
        self.rm_len = 0;
        self.rm_overflow = false;
        self.saved();
        Ok(())
    }

    fn saved(&mut self) {
        self.saves += 1;
        self.win_saves += 1;
        self.pending = 0;
        self.last_save = self.frame;
    }

    fn mark(&mut self, id: usize, fresh: bool) {
        self.pending += 1;
        self.dirty |= 1 << id;
        if fresh {
            self.fresh |= 1 << id;
        }
    }

    pub fn create(&mut self, name: &[u8], owner: u8) -> Result<(), FsError> {
        self.fs.create(name, owner)?;
        let id = self.fs.find_name(name).ok_or(FsError::NotFound)?;
        self.mark(id, true);
        Ok(())
    }

    pub fn mkdir(&mut self, name: &[u8], owner: u8) -> Result<(), FsError> {
        self.fs.mkdir(name, owner)?;
        let id = self.fs.find_name(name).ok_or(FsError::NotFound)?;
        self.mark(id, true);
        Ok(())
    }

    pub fn write(&mut self, name: &[u8], caller: u8, grant: usize, data: &[u8]) -> Result<(), FsError> {
        let id = self.fs.find_name(name).ok_or(FsError::NotFound)?;
        self.fs.write(id, caller, grant, data)?;
        self.mark(id, false);
        Ok(())
    }

    pub fn remove(&mut self, name: &[u8], caller: u8) -> Result<(), FsError> {
        let id = self.fs.find_name(name).ok_or(FsError::NotFound)?;
        let owner = self.fs.info(id).map_or(0, |x| x.1);
        self.fs.remove(id, caller)?;
        self.pending += 1;
        self.dirty &= !(1 << id);
        self.fresh &= !(1 << id);
        if self.rm_len < RM_MAX {
            let mut nm = [0u8; NAME_MAX];
            nm[..name.len()].copy_from_slice(name);
            self.removed[self.rm_len] = (nm, name.len() as u8, owner);
            self.rm_len += 1;
        } else {
            self.rm_overflow = true;
        }
        Ok(())
    }

    pub fn read(&self, name: &[u8], caller: u8, out: &mut [u8]) -> Result<usize, FsError> {
        let id = self.fs.find_name(name).ok_or(FsError::NotFound)?;
        self.fs.read(id, caller, out)
    }

    /// Chiamata a ogni frame. Salva a soglia o a tempo, solo se c'è almeno una modifica
    /// e se la finestra di un minuto non ha già raggiunto il limite di salvataggi.
    pub fn frame(&mut self) -> Result<(), FsError> {
        self.frame = self.frame.wrapping_add(1);
        if self.frame.wrapping_sub(self.win_start) >= 3600 {
            self.win_start = self.frame;
            self.win_saves = 0;
        }
        if self.pending == 0 || self.win_saves >= self.profile.max_writes_per_min {
            return Ok(());
        }
        let by_units = self.pending >= self.profile.units.max(1);
        let by_time = self.profile.max_frames > 0 && self.frame.wrapping_sub(self.last_save) >= self.profile.max_frames;
        if by_units || by_time {
            self.sync()?;
        }
        Ok(())
    }

    pub fn fs(&self) -> &Fs {
        &self.fs
    }

    pub fn into_medium(self) -> M {
        self.m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Ram(Vec<u8>);
    impl Medium for Ram {
        fn read(&self, off: usize, out: &mut [u8]) {
            out.copy_from_slice(&self.0[off..off + out.len()]);
        }
        fn write(&mut self, off: usize, d: &[u8]) {
            self.0[off..off + d.len()].copy_from_slice(d);
        }
    }

    struct Counting {
        inner: Ram,
        writes: usize,
        profile: Profile,
    }
    impl Medium for Counting {
        fn read(&self, off: usize, out: &mut [u8]) {
            self.inner.read(off, out);
        }
        fn write(&mut self, off: usize, d: &[u8]) {
            self.writes += 1;
            self.inner.write(off, d);
        }
        fn profile(&self) -> Profile {
            self.profile
        }
    }

    const R: usize = 1024;
    fn fresh() -> Ram {
        Ram(vec![0xFF; HDR_AREA + 2 * R])
    }

    /// Profilo GBA: soglia 64 modifiche o 30 s (1800 frame), al massimo 2 salvataggi al minuto. Da verificare su hardware.
    const GBA_PROFILE: Profile = Profile { units: 64, max_frames: 1800, max_writes_per_min: 2 };

    fn prof(units: u32, max_frames: u32, max: u32) -> Profile {
        Profile { units, max_frames, max_writes_per_min: max }
    }

    #[test]
    fn no_save_without_changes() {
        let mut p = Persist::open(Counting { inner: fresh(), writes: 0, profile: prof(1, 1, u32::MAX) }, R);
        let base = p.saves;
        let wbase = p.m.writes;
        for _ in 0..500 {
            p.frame().unwrap();
        }
        assert_eq!(p.saves, base);
        assert_eq!(p.m.writes, wbase);
    }

    #[test]
    fn saves_at_unit_threshold() {
        let mut p = Persist::open(Counting { inner: fresh(), writes: 0, profile: prof(5, 0, u32::MAX) }, R);
        let base = p.saves;
        p.create(b"f", 0).unwrap(); // 1
        for _ in 0..3 {
            p.write(b"f", 0, 1024, b"x").unwrap(); // 2..4
            p.frame().unwrap();
        }
        assert_eq!(p.saves, base);
        p.write(b"f", 0, 1024, b"x").unwrap(); // 5
        p.frame().unwrap();
        assert_eq!(p.saves, base + 1);
    }

    #[test]
    fn saves_at_time_threshold() {
        let mut p = Persist::open(Counting { inner: fresh(), writes: 0, profile: prof(1000, 120, u32::MAX) }, R);
        let base = p.saves;
        p.create(b"f", 0).unwrap();
        for _ in 0..119 {
            p.frame().unwrap();
        }
        assert_eq!(p.saves, base);
        p.frame().unwrap();
        assert_eq!(p.saves, base + 1);
        // Senza nuove modifiche non salva più.
        for _ in 0..300 {
            p.frame().unwrap();
        }
        assert_eq!(p.saves, base + 1);
    }

    #[test]
    fn respects_writes_per_minute_limit() {
        let mut p = Persist::open(fresh(), R);
        p.profile = prof(1, 1, 2);
        p.create(b"f", 0).unwrap();
        for i in 0..600u32 {
            p.write(b"f", 0, 1024, &[(i & 0xFF) as u8; 8]).unwrap();
            p.frame().unwrap();
        }
        assert!(p.saves <= 2, "salvataggi: {}", p.saves);
    }

    #[test]
    fn survives_reopen_after_sync() {
        let mut p = Persist::open(fresh(), R);
        p.create(b"a", 1).unwrap();
        p.write(b"a", 1, 1024, b"hello").unwrap();
        p.sync().unwrap();
        let p2 = Persist::open(p.into_medium(), R);
        let mut out = [0u8; 16];
        let n = p2.read(b"a", 1, &mut out).unwrap();
        assert_eq!(&out[..n], b"hello");
    }

    #[test]
    fn unsynced_changes_are_lost() {
        let mut p = Persist::open(fresh(), R);
        p.create(b"a", 0).unwrap();
        p.sync().unwrap();
        p.write(b"a", 0, 1024, b"new").unwrap();
        let p2 = Persist::open(p.into_medium(), R);
        let mut out = [0u8; 4];
        assert_eq!(p2.read(b"a", 0, &mut out), Ok(0));
    }

    #[test]
    fn coalesces_many_writes_into_one_save() {
        let mut p = Persist::open(fresh(), R);
        p.create(b"f", 0).unwrap();
        for i in 0..100u8 {
            p.write(b"f", 0, 1024, &[i; 20]).unwrap();
        }
        let before = p.saves;
        p.sync().unwrap();
        assert_eq!(p.saves - before, 1);
    }

    #[test]
    fn compaction_keeps_latest() {
        let mut p = Persist::open(fresh(), R);
        p.create(b"f", 0).unwrap();
        for i in 0..200u8 {
            p.write(b"f", 0, 1024, &[i; 100]).unwrap();
            p.sync().unwrap();
        }
        assert!(p.gen > 2);
        let p2 = Persist::open(p.into_medium(), R);
        let mut out = [0u8; 100];
        assert_eq!(p2.read(b"f", 0, &mut out), Ok(100));
        assert_eq!(out, [199u8; 100]);
    }

    #[test]
    fn torn_tail_is_ignored() {
        let mut p = Persist::open(fresh(), R);
        p.create(b"a", 0).unwrap();
        p.write(b"a", 0, 1024, b"one").unwrap();
        p.sync().unwrap();
        p.write(b"a", 0, 1024, &[7u8; 300]).unwrap();
        p.sync().unwrap();
        let last = HDR_AREA + p.active * R + p.end - 1;
        let mut m = p.into_medium();
        m.0[last] ^= 0xFF;
        let p2 = Persist::open(m, R);
        let mut out = [0u8; 300];
        let n = p2.read(b"a", 0, &mut out).unwrap();
        assert_eq!(&out[..n], b"one");
    }

    #[test]
    fn remove_then_recreate_same_name() {
        let mut p = Persist::open(fresh(), R);
        p.create(b"a", 0).unwrap();
        p.write(b"a", 0, 1024, b"old").unwrap();
        p.sync().unwrap();
        p.remove(b"a", 0).unwrap();
        p.create(b"a", 0).unwrap();
        p.write(b"a", 0, 1024, b"new").unwrap();
        p.sync().unwrap();
        let p2 = Persist::open(p.into_medium(), R);
        let mut out = [0u8; 8];
        let n = p2.read(b"a", 0, &mut out).unwrap();
        assert_eq!(&out[..n], b"new");
    }

    #[test]
    fn gba_profile_stays_under_save_limit() {
        let c = Counting { inner: fresh(), writes: 0, profile: GBA_PROFILE };
        let mut p = Persist::open(c, R);
        p.create(b"f", 0).unwrap();
        p.sync().unwrap();
        let before = p.saves;
        // Un minuto di gioco (3600 frame) con una modifica per frame.
        for i in 0..3600u32 {
            p.write(b"f", 0, 1024, &[(i & 0xFF) as u8; 20]).unwrap();
            p.frame().unwrap();
        }
        let per_min = p.saves - before;
        assert!(per_min <= GBA_PROFILE.max_writes_per_min, "salvataggi al minuto: {per_min}");
        assert!(per_min >= 1, "nessun salvataggio");
    }

    #[test]
    fn powerful_profile_saves_fast() {
        let mut p = Persist::open(Counting { inner: fresh(), writes: 0, profile: POWERFUL }, R);
        let base = p.saves;
        p.create(b"f", 0).unwrap();
        for _ in 0..3 {
            p.write(b"f", 0, 1024, b"x").unwrap();
        }
        p.frame().unwrap();
        assert_eq!(p.saves, base + 1);
    }

    #[test]
    fn directories_survive_reopen() {
        let mut p = Persist::open(fresh(), R);
        p.mkdir(b"docs", 0).unwrap();
        p.create(b"docs/a", 0).unwrap();
        p.write(b"docs/a", 0, 1024, b"hi").unwrap();
        p.sync().unwrap();
        let p2 = Persist::open(p.into_medium(), R);
        let mut out = [0u8; 2];
        assert_eq!(p2.read(b"docs/a", 0, &mut out), Ok(2));
        assert_eq!(&out, b"hi");
    }

    #[test]
    fn continuous_profile_is_incremental() {
        let mut p = Persist::open(fresh(), R);
        p.create(b"f", 0).unwrap();
        p.write(b"f", 0, 1024, &[1u8; 50]).unwrap();
        p.frame().unwrap();
        let before = p.end;
        p.write(b"f", 0, 1024, &[2u8; 50]).unwrap();
        p.frame().unwrap();
        // Ogni salvataggio aggiunge solo i record dei file cambiati, non riscrive il log.
        assert_eq!(p.end - before, rec_len(1, 50));
    }
}
