//! Modello schermo a tre oggetti (docs/kernel-study.md, riga 2): Plane, Output, Pipe.
//! Logica pura, no_std, senza heap. Una Pipe<N> lega fino a N piani a un output.
#![cfg_attr(not(test), no_std)]

/// Formato dei pixel: serve solo per il controllo del pitch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Xrgb8888,
    Rgb565,
}

impl Format {
    pub const fn bpp(self) -> u32 {
        match self {
            Format::Xrgb8888 => 4,
            Format::Rgb565 => 2,
        }
    }
}

/// Risoluzione richiesta dall'output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
}

/// Piano: sorgente dei pixel. `buf` è l'indice del buffer, noto al chiamante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plane {
    pub format: Format,
    pub pitch: u32,
    pub width: u32,
    pub height: u32,
    pub buf: u32,
}

/// Output: dove vanno i pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Output {
    pub mode: Mode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Pitch sotto width * bytes_per_pixel.
    BadPitch,
    /// Dimensioni del piano diverse dal mode dell'output.
    BadSize,
    /// Questo buffer è già legato alla pipe.
    AlreadyBound,
    /// Tutti gli slot sono occupati.
    Full,
    /// Slot vuoto o fuori range.
    NotBound,
}

impl Plane {
    /// Controlla che il piano possa essere mostrato sull'output.
    pub fn check(&self, out: &Output) -> Result<(), Error> {
        let min = self.width.checked_mul(self.format.bpp()).ok_or(Error::BadPitch)?;
        if self.pitch < min {
            return Err(Error::BadPitch);
        }
        if self.width != out.mode.width || self.height != out.mode.height {
            return Err(Error::BadSize);
        }
        Ok(())
    }
}

/// Pipe: lega fino a N piani all'output. Ogni piano occupa uno slot.
pub struct Pipe<const N: usize> {
    pub output: Output,
    slots: [Option<Plane>; N],
}

impl<const N: usize> Pipe<N> {
    pub const fn new(output: Output) -> Self {
        Pipe { output, slots: [None; N] }
    }

    /// Lega un piano valido; ritorna lo slot occupato.
    pub fn bind(&mut self, plane: Plane) -> Result<usize, Error> {
        plane.check(&self.output)?;
        if self.planes().any(|p| p.buf == plane.buf) {
            return Err(Error::AlreadyBound);
        }
        let slot = self.slots.iter().position(Option::is_none).ok_or(Error::Full)?;
        self.slots[slot] = Some(plane);
        Ok(slot)
    }

    /// Scioglie il legame di uno slot e ritorna il piano.
    pub fn unbind(&mut self, slot: usize) -> Result<Plane, Error> {
        self.slots.get_mut(slot).and_then(Option::take).ok_or(Error::NotBound)
    }

    /// Piani legati in questo momento.
    pub fn planes(&self) -> impl Iterator<Item = &Plane> {
        self.slots.iter().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OUT: Output = Output { mode: Mode { width: 640, height: 480 } };

    fn plane(buf: u32) -> Plane {
        Plane { format: Format::Xrgb8888, pitch: 640 * 4, width: 640, height: 480, buf }
    }

    #[test]
    fn bind_ok() {
        let mut pipe = Pipe::<4>::new(OUT);
        assert_eq!(pipe.bind(plane(1)), Ok(0));
        assert_eq!(pipe.planes().count(), 1);
    }

    #[test]
    fn bad_pitch_rejected() {
        let mut pipe = Pipe::<4>::new(OUT);
        let p = Plane { pitch: 640 * 4 - 1, ..plane(1) };
        assert_eq!(pipe.bind(p), Err(Error::BadPitch));
        assert_eq!(pipe.planes().count(), 0);
    }

    #[test]
    fn bad_size_rejected() {
        let mut pipe = Pipe::<4>::new(OUT);
        let p = Plane { height: 479, ..plane(1) };
        assert_eq!(pipe.bind(p), Err(Error::BadSize));
    }

    #[test]
    fn double_bind_rejected() {
        let mut pipe = Pipe::<4>::new(OUT);
        assert_eq!(pipe.bind(plane(1)), Ok(0));
        assert_eq!(pipe.bind(plane(1)), Err(Error::AlreadyBound));
        assert_eq!(pipe.planes().count(), 1);
    }

    #[test]
    fn unbind_frees_slot() {
        let mut pipe = Pipe::<2>::new(OUT);
        let slot = pipe.bind(plane(1)).unwrap();
        assert_eq!(pipe.unbind(slot), Ok(plane(1)));
        assert_eq!(pipe.unbind(slot), Err(Error::NotBound));
        assert_eq!(pipe.bind(plane(1)), Ok(0));
    }

    #[test]
    fn holds_n_planes() {
        let mut pipe = Pipe::<4>::new(OUT);
        for b in 0..4 {
            assert!(pipe.bind(plane(b)).is_ok());
        }
        assert_eq!(pipe.bind(plane(4)), Err(Error::Full));
    }
}
