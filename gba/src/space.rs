//! Due spazi separati nello stesso supporto: sistema (formattabile) e utente (mai toccato da una formattazione).
//! Ogni spazio è una finestra sul supporto con la sua regione di log, le sue intestazioni e i suoi checksum.
//! Il supporto è condiviso tramite `RefCell`: nessun unsafe, solo `core`.
#![allow(dead_code)]
use crate::fs::FsError;
use crate::persist::{Medium, Persist, Profile, HDR_AREA};
use core::cell::RefCell;

/// Capacità dell'agente di sistema: unico chiamante che può formattare lo spazio di sistema.
pub const SYS_CAP: u8 = 0;

/// Finestra [base, base+len) su un supporto condiviso.
pub struct Window<'a, M: Medium> {
    dev: &'a RefCell<M>,
    base: usize,
    len: usize,
}

impl<'a, M: Medium> Window<'a, M> {
    pub fn new(dev: &'a RefCell<M>, base: usize, len: usize) -> Self {
        Self { dev, base, len }
    }
}

impl<'a, M: Medium> Medium for Window<'a, M> {
    fn read(&self, off: usize, out: &mut [u8]) {
        assert!(off + out.len() <= self.len, "lettura fuori dalla finestra");
        self.dev.borrow().read(self.base + off, out);
    }
    fn write(&mut self, off: usize, data: &[u8]) {
        assert!(off + data.len() <= self.len, "scrittura fuori dalla finestra");
        self.dev.borrow_mut().write(self.base + off, data);
    }
    fn profile(&self) -> Profile {
        self.dev.borrow().profile()
    }
}

pub struct Spaces<'a, M: Medium> {
    dev: &'a RefCell<M>,
    sys_len: usize,
    usr_len: usize,
    pub sys: Persist<Window<'a, M>>,
    pub usr: Persist<Window<'a, M>>,
}

impl<'a, M: Medium> Spaces<'a, M> {
    /// Lo spazio di sistema occupa [0, sys_len), quello utente [sys_len, sys_len + usr_len).
    pub fn open(dev: &'a RefCell<M>, sys_len: usize, usr_len: usize) -> Self {
        let sys = Persist::open(Window::new(dev, 0, sys_len), (sys_len - HDR_AREA) / 2);
        let usr = Persist::open(Window::new(dev, sys_len, usr_len), (usr_len - HDR_AREA) / 2);
        Self { dev, sys_len, usr_len, sys, usr }
    }

    /// Formatta lo spazio di sistema: azzera le sue intestazioni e lo riapre vuoto.
    /// Non tocca la finestra utente. Solo la capacità di sistema può formattare.
    pub fn format_system(&mut self, caller: u8) -> Result<(), FsError> {
        if caller != SYS_CAP {
            return Err(FsError::Denied);
        }
        Window::new(self.dev, 0, self.sys_len).write(0, &[0u8; HDR_AREA]);
        self.sys = Persist::open(Window::new(self.dev, 0, self.sys_len), (self.sys_len - HDR_AREA) / 2);
        Ok(())
    }

    /// Lunghezza della finestra utente, per le verifiche.
    pub fn usr_len(&self) -> usize {
        self.usr_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc_vec::Ram;

    mod alloc_vec {
        use crate::persist::{Medium, Profile};
        pub struct Ram(pub Vec<u8>);
        impl Medium for Ram {
            fn read(&self, off: usize, out: &mut [u8]) {
                out.copy_from_slice(&self.0[off..off + out.len()]);
            }
            fn write(&mut self, off: usize, d: &[u8]) {
                self.0[off..off + d.len()].copy_from_slice(d);
            }
            fn profile(&self) -> Profile {
                Profile { units: 1, max_frames: 1, max_writes_per_min: u32::MAX }
            }
        }
    }

    const SYS: usize = 4096;
    const USR: usize = 4096;

    fn device() -> RefCell<Ram> {
        RefCell::new(Ram(vec![0xFF; SYS + USR]))
    }

    #[test]
    fn format_system_keeps_user() {
        let dev = device();
        {
            let mut s = Spaces::open(&dev, SYS, USR);
            s.sys.create(b"kernel", 0).unwrap();
            s.sys.write(b"kernel", 0, 512, b"k").unwrap();
            s.sys.sync().unwrap();
            s.usr.create(b"note", 1).unwrap();
            s.usr.write(b"note", 1, 512, b"mine").unwrap();
            s.usr.sync().unwrap();

            s.format_system(SYS_CAP).unwrap();
            assert_eq!(s.sys.read(b"kernel", 0, &mut [0u8; 1]), Err(FsError::NotFound));
            let mut out = [0u8; 4];
            assert_eq!(s.usr.read(b"note", 1, &mut out), Ok(4));
            assert_eq!(&out, b"mine");
        }
        // Riavvio: l'utente è ancora lì, il sistema è vuoto.
        let s2 = Spaces::open(&dev, SYS, USR);
        let mut out = [0u8; 4];
        assert_eq!(s2.usr.read(b"note", 1, &mut out), Ok(4));
        assert_eq!(&out, b"mine");
        assert_eq!(s2.sys.read(b"kernel", 0, &mut [0u8; 1]), Err(FsError::NotFound));
    }

    #[test]
    fn format_needs_system_capability() {
        let dev = device();
        let mut s = Spaces::open(&dev, SYS, USR);
        s.sys.create(b"kernel", 0).unwrap();
        s.sys.sync().unwrap();
        assert_eq!(s.format_system(1), Err(FsError::Denied));
        assert!(s.sys.read(b"kernel", 0, &mut [0u8; 1]).is_ok());
    }

    #[test]
    fn user_space_works_after_format() {
        let dev = device();
        let mut s = Spaces::open(&dev, SYS, USR);
        s.format_system(SYS_CAP).unwrap();
        s.usr.create(b"after", 2).unwrap();
        s.usr.write(b"after", 2, 512, b"ok").unwrap();
        s.usr.sync().unwrap();
        let s2 = Spaces::open(&dev, SYS, USR);
        let mut out = [0u8; 2];
        assert_eq!(s2.usr.read(b"after", 2, &mut out), Ok(2));
        assert_eq!(&out, b"ok");
    }
}
