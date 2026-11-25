//! Archiviatore: contenitore senza compressione. Formato:
//! `"HAR1"`, poi per ogni file: nome (1 byte di lunghezza + nome), dati (2 byte LE + dati).
//! La cifratura (Ascon) non è ancora qui: va sopra al contenitore, misurata nel simulatore.

pub const MAGIC: &[u8; 4] = b"HAR1";
pub const NAME_MAX: usize = 24;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Err {
    Full,
    NameTooLong,
    DataTooLong,
    BadMagic,
    Truncated,
}

/// Scrive i file in `out`. Ritorna la lunghezza scritta.
pub fn pack(files: &[(&[u8], &[u8])], out: &mut [u8]) -> Result<usize, Err> {
    let mut at = 0;
    put(out, &mut at, MAGIC)?;
    for (name, data) in files {
        if name.len() > NAME_MAX {
            return Err(Err::NameTooLong);
        }
        if data.len() > u16::MAX as usize {
            return Err(Err::DataTooLong);
        }
        put(out, &mut at, &[name.len() as u8])?;
        put(out, &mut at, name)?;
        put(out, &mut at, &(data.len() as u16).to_le_bytes())?;
        put(out, &mut at, data)?;
    }
    Ok(at)
}

fn put(out: &mut [u8], at: &mut usize, b: &[u8]) -> Result<(), Err> {
    let end = *at + b.len();
    if end > out.len() {
        return Err(Err::Full);
    }
    out[*at..end].copy_from_slice(b);
    *at = end;
    Ok(())
}

/// Lettore dei file di un archivio, uno alla volta.
pub struct Reader<'a> {
    arc: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(arc: &'a [u8]) -> Result<Self, Err> {
        if arc.len() < 4 || &arc[..4] != MAGIC {
            return Err(Err::BadMagic);
        }
        Ok(Reader { arc, at: 4 })
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], Err> {
        let end = self.at.checked_add(n).ok_or(Err::Truncated)?;
        if end > self.arc.len() {
            return Err(Err::Truncated);
        }
        let s = &self.arc[self.at..end];
        self.at = end;
        Ok(s)
    }

    /// Prossimo file (nome, dati). `None` a fine archivio.
    pub fn next_file(&mut self) -> Option<Result<(&'a [u8], &'a [u8]), Err>> {
        if self.at == self.arc.len() {
            return None;
        }
        Some(self.read_one())
    }

    fn read_one(&mut self) -> Result<(&'a [u8], &'a [u8]), Err> {
        let n = self.take(1)?[0] as usize;
        let name = self.take(n)?;
        let l = self.take(2)?;
        let len = u16::from_le_bytes([l[0], l[1]]) as usize;
        let data = self.take(len)?;
        Ok((name, data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn andata_e_ritorno() {
        let mut out = [0u8; 256];
        let n = pack(&[(b"a.txt", b"ciao"), (b"docs/hello", b"")], &mut out).unwrap();
        let mut r = Reader::new(&out[..n]).unwrap();
        assert_eq!(r.next_file().unwrap().unwrap(), (&b"a.txt"[..], &b"ciao"[..]));
        assert_eq!(r.next_file().unwrap().unwrap(), (&b"docs/hello"[..], &b""[..]));
        assert!(r.next_file().is_none());
    }

    #[test]
    fn limiti_e_errori() {
        let mut piccolo = [0u8; 8];
        assert_eq!(pack(&[(b"nome", b"dati lunghi")], &mut piccolo), Err(Err::Full));
        let lungo = [b'x'; NAME_MAX + 1];
        assert_eq!(pack(&[(&lungo, b"")], &mut [0u8; 64]), Err(Err::NameTooLong));
        assert_eq!(Reader::new(b"XXXX").err(), Some(Err::BadMagic));
        let mut out = [0u8; 64];
        let n = pack(&[(b"a", b"0123456789")], &mut out).unwrap();
        let mut r = Reader::new(&out[..n - 3]).unwrap();
        assert_eq!(r.next_file().unwrap(), Err(Err::Truncated));
    }
}
