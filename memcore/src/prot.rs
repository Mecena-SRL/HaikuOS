//! Regola W^X (riga 14): una pagina non è mai scrivibile ed eseguibile insieme.
//! Il tipo non ha una variante W+X: la richiesta viene rifiutata in `new`.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prot {
    Read,
    ReadWrite,
    ReadExec,
}

impl Prot {
    /// `None` se si chiedono scrittura ed esecuzione insieme.
    pub const fn new(write: bool, exec: bool) -> Option<Prot> {
        match (write, exec) {
            (true, true) => None,
            (true, false) => Some(Prot::ReadWrite),
            (false, true) => Some(Prot::ReadExec),
            (false, false) => Some(Prot::Read),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Prot;

    #[test]
    fn wx_is_rejected() {
        assert_eq!(Prot::new(true, true), None);
        assert_eq!(Prot::new(true, false), Some(Prot::ReadWrite));
        assert_eq!(Prot::new(false, true), Some(Prot::ReadExec));
        assert_eq!(Prot::new(false, false), Some(Prot::Read));
    }
}
