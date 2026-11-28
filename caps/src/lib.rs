//! Promesse per processo, stile pledge (kernel-study riga 15). Un insieme di permessi a bit,
//! fissato all'avvio dal manifest. Può solo restringersi, mai allargarsi. Puro, senza I/O.
#![cfg_attr(not(test), no_std)]

/// Il permesso non è nell'insieme (stesso esito della quota negata).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Denied;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caps(u32);

impl Caps {
    pub const DISPLAY: Caps = Caps(1 << 0);
    pub const INPUT: Caps = Caps(1 << 1);
    pub const AUDIO: Caps = Caps(1 << 2);
    pub const FS_READ: Caps = Caps(1 << 3);
    pub const FS_WRITE: Caps = Caps(1 << 4);
    pub const NET: Caps = Caps(1 << 5);
    pub const SPAWN: Caps = Caps(1 << 6);

    pub const fn none() -> Caps {
        Caps(0)
    }

    /// Tutti i permessi: solo per il monolite.
    pub const fn all() -> Caps {
        Caps(0x7f)
    }

    pub const fn union(self, other: Caps) -> Caps {
        Caps(self.0 | other.0)
    }

    /// Vero se tutti i bit di `c` sono presenti.
    pub const fn has(self, c: Caps) -> bool {
        self.0 & c.0 == c.0
    }

    /// Restringe: il risultato contiene solo i permessi presenti in entrambi.
    /// Un figlio usa `parent.restrict(manifest)`: non può ottenere più del padre.
    pub const fn restrict(self, keep: Caps) -> Caps {
        Caps(self.0 & keep.0)
    }

    pub fn require(self, c: Caps) -> Result<(), Denied> {
        if self.has(c) { Ok(()) } else { Err(Denied) }
    }

    pub const fn bits(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restringe_e_non_allarga() {
        let padre = Caps::DISPLAY.union(Caps::INPUT);
        let figlio = padre.restrict(Caps::INPUT.union(Caps::NET));
        assert!(figlio.has(Caps::INPUT));
        assert!(!figlio.has(Caps::NET));
        assert!(!figlio.has(Caps::DISPLAY));
        // Anche chiedendo tutto, il figlio non supera il padre.
        assert_eq!(figlio.restrict(Caps::all()), figlio);
    }

    #[test]
    fn negato_senza_permesso() {
        let c = Caps::FS_READ;
        assert_eq!(c.require(Caps::FS_READ), Ok(()));
        assert_eq!(c.require(Caps::FS_WRITE), Err(Denied));
        assert_eq!(Caps::none().require(Caps::SPAWN), Err(Denied));
    }

    #[test]
    fn all_copre_ogni_bit_noto() {
        for c in [Caps::DISPLAY, Caps::INPUT, Caps::AUDIO, Caps::FS_READ, Caps::FS_WRITE, Caps::NET, Caps::SPAWN] {
            assert!(Caps::all().has(c));
        }
    }
}
