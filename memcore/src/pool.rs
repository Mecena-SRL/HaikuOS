//! Pool a dimensione fissa per oggetti del kernel (riga 13).
//! N slot decisi a compilazione: nessun heap, nessuna frammentazione.

pub struct Pool<T, const N: usize> {
    slots: [Option<T>; N],
}

impl<T, const N: usize> Pool<T, N> {
    pub const fn new() -> Self {
        Self { slots: [const { None }; N] }
    }

    /// Inserisce `v` nel primo slot libero e ne restituisce l'indice.
    pub fn alloc(&mut self, v: T) -> Option<usize> {
        let i = self.slots.iter().position(|s| s.is_none())?;
        self.slots[i] = Some(v);
        Some(i)
    }

    pub fn get(&self, i: usize) -> Option<&T> {
        self.slots.get(i)?.as_ref()
    }

    pub fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        self.slots.get_mut(i)?.as_mut()
    }

    /// Libera lo slot `i` e restituisce il valore.
    pub fn free(&mut self, i: usize) -> Option<T> {
        self.slots.get_mut(i)?.take()
    }
}

#[cfg(test)]
mod tests {
    use super::Pool;

    #[test]
    fn fills_then_reuses_slots() {
        let mut p: Pool<u32, 2> = Pool::new();
        assert_eq!(p.alloc(10), Some(0));
        assert_eq!(p.alloc(20), Some(1));
        assert_eq!(p.alloc(30), None);
        assert_eq!(p.free(0), Some(10));
        assert_eq!(p.free(0), None);
        assert_eq!(p.alloc(30), Some(0));
        assert_eq!(p.get(0), Some(&30));
        *p.get_mut(1).unwrap() += 1;
        assert_eq!(p.get(1), Some(&21));
        assert_eq!(p.get(5), None);
    }
}
