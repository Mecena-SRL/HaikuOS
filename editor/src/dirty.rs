//! Righe sporche della griglia: si ridisegnano solo quelle cambiate.
//! Righe oltre la 63 usano il ridisegno completo (conservativo, nessuna allocazione).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dirty {
    all: bool,
    rows: u64,
}

impl Dirty {
    /// Nuovo: tutto sporco, così il primo frame è completo.
    pub const fn new() -> Self {
        Self { all: true, rows: 0 }
    }

    pub fn mark_row(&mut self, y: usize) {
        if y < 64 {
            self.rows |= 1 << y;
        } else {
            self.all = true;
        }
    }

    pub fn mark_all(&mut self) {
        self.all = true;
    }

    pub fn is_dirty(&self, y: usize) -> bool {
        self.all || (y < 64 && self.rows >> y & 1 == 1)
    }

    pub fn clear(&mut self) {
        self.all = false;
        self.rows = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_and_clears_rows() {
        let mut d = Dirty::new();
        d.clear();
        assert!(!d.is_dirty(3));
        d.mark_row(3);
        assert!(d.is_dirty(3) && !d.is_dirty(2));
        d.mark_row(100); // oltre 63: ridisegno completo
        assert!(d.is_dirty(0));
        d.clear();
        assert!(!d.is_dirty(0));
    }
}
