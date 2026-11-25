//! Stringa a lunghezza fissa, utilizzabile senza allocatore.

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Text<const N: usize> {
    len: usize,
    buf: [u8; N],
}

impl<const N: usize> Text<N> {
    pub const fn empty() -> Self {
        Self { len: 0, buf: [0; N] }
    }

    /// `None` se il dato supera la capacità `N`.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > N {
            return None;
        }
        let mut t = Self::empty();
        t.buf[..bytes.len()].copy_from_slice(bytes);
        t.len = bytes.len();
        Some(t)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).unwrap_or("")
    }
}

impl<const N: usize> Text<N> {
    /// Azzera il contenuto con scritture volatili (es. per le password).
    pub fn wipe(&mut self) {
        wipe_bytes(&mut self.buf);
        self.len = 0;
    }
}

/// Azzera un buffer con scritture volatili, così il compilatore non le elimina.
pub fn wipe_bytes(buf: &mut [u8]) {
    for b in buf.iter_mut() {
        // SAFETY: `b` è un riferimento valido e allineato a un byte.
        unsafe { core::ptr::write_volatile(b, 0) };
    }
}
