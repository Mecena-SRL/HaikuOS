//! Documento di testo a dimensione fissa. Solo `core`, nessuna allocazione.
//! Formato su disco: byte puri, a capo LF. In lettura CRLF e CR diventano LF.
//! Il cursore è un indice di byte; le righe sono separate da LF.

#[derive(Debug, PartialEq, Eq)]
pub enum TextError {
    /// Il documento non ha più spazio.
    Full,
    /// Carattere non ammesso (CR in inserimento: si usa LF).
    BadChar,
}

pub struct Text<const N: usize> {
    buf: [u8; N],
    len: usize,
    cur: usize,
}

impl<const N: usize> Text<N> {
    pub const fn new() -> Self {
        Text { buf: [0; N], len: 0, cur: 0 }
    }

    /// Contenuto attuale (LF per a capo).
    pub fn bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn cursor(&self) -> usize {
        self.cur
    }

    /// Carica da file. Se non entra, il documento resta vuoto.
    pub fn load(&mut self, src: &[u8]) -> Result<(), TextError> {
        self.len = 0;
        self.cur = 0;
        let r = self.fill(src);
        if r.is_err() {
            self.len = 0;
        }
        r
    }

    fn fill(&mut self, src: &[u8]) -> Result<(), TextError> {
        let mut i = 0;
        while i < src.len() {
            let b = src[i];
            i += 1;
            if b == b'\r' {
                if i < src.len() && src[i] == b'\n' {
                    i += 1;
                }
                self.append(b'\n')?;
            } else {
                self.append(b)?;
            }
        }
        Ok(())
    }

    fn append(&mut self, b: u8) -> Result<(), TextError> {
        if self.len == N {
            return Err(TextError::Full);
        }
        self.buf[self.len] = b;
        self.len += 1;
        Ok(())
    }

    /// Inserisce un byte alla posizione del cursore e avanza.
    pub fn insert(&mut self, b: u8) -> Result<(), TextError> {
        if b == b'\r' {
            return Err(TextError::BadChar);
        }
        if self.len == N {
            return Err(TextError::Full);
        }
        self.buf.copy_within(self.cur..self.len, self.cur + 1);
        self.buf[self.cur] = b;
        self.len += 1;
        self.cur += 1;
        Ok(())
    }

    /// Cancella il byte prima del cursore.
    pub fn backspace(&mut self) -> bool {
        if self.cur == 0 {
            return false;
        }
        self.buf.copy_within(self.cur..self.len, self.cur - 1);
        self.cur -= 1;
        self.len -= 1;
        true
    }

    /// Cancella il byte sotto il cursore.
    pub fn delete(&mut self) -> bool {
        if self.cur >= self.len {
            return false;
        }
        self.buf.copy_within(self.cur + 1..self.len, self.cur);
        self.len -= 1;
        true
    }

    pub fn left(&mut self) -> bool {
        if self.cur == 0 {
            return false;
        }
        self.cur -= 1;
        true
    }

    pub fn right(&mut self) -> bool {
        if self.cur >= self.len {
            return false;
        }
        self.cur += 1;
        true
    }

    fn line_start(&self, pos: usize) -> usize {
        let mut p = pos;
        while p > 0 && self.buf[p - 1] != b'\n' {
            p -= 1;
        }
        p
    }

    fn line_end(&self, pos: usize) -> usize {
        let mut p = pos;
        while p < self.len && self.buf[p] != b'\n' {
            p += 1;
        }
        p
    }

    /// Su una riga; mantiene la colonna, si la riga è più corta va a fine riga.
    pub fn up(&mut self) -> bool {
        let start = self.line_start(self.cur);
        if start == 0 {
            return false;
        }
        let col = self.cur - start;
        let prev_end = start - 1;
        let prev_start = self.line_start(prev_end);
        self.cur = (prev_start + col).min(prev_end);
        true
    }

    /// Giù di una riga; mantiene la colonna.
    pub fn down(&mut self) -> bool {
        let col = self.cur - self.line_start(self.cur);
        let end = self.line_end(self.cur);
        if end >= self.len {
            return false;
        }
        let next_start = end + 1;
        self.cur = (next_start + col).min(self.line_end(next_start));
        true
    }

    /// Riga e colonna del cursore, a partire da zero.
    pub fn line_col(&self) -> (usize, usize) {
        let start = self.line_start(self.cur);
        let line = self.buf[..start].iter().filter(|&&b| b == b'\n').count();
        (line, self.cur - start)
    }

    /// Numero di righe (una riga vuota conta come riga).
    pub fn lines(&self) -> usize {
        self.buf[..self.len].iter().filter(|&&b| b == b'\n').count() + 1
    }

    /// Testo della riga `idx`, senza LF.
    pub fn line(&self, idx: usize) -> Option<&[u8]> {
        let mut start = 0;
        let mut n = 0;
        loop {
            let end = self.line_end(start);
            if n == idx {
                return Some(&self.buf[start..end]);
            }
            if end >= self.len {
                return None;
            }
            start = end + 1;
            n += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_normalizza_a_capo() {
        let mut t: Text<64> = Text::new();
        t.load(b"a\r\nb\rc\nd").unwrap();
        assert_eq!(t.bytes(), b"a\nb\nc\nd");
        assert_eq!(t.cursor(), 0);
        assert_eq!(t.lines(), 4);
    }

    #[test]
    fn load_troppo_grande_lascia_vuoto() {
        let mut t: Text<4> = Text::new();
        assert_eq!(t.load(b"abcde"), Err(TextError::Full));
        assert_eq!(t.len(), 0);
    }

    #[test]
    fn inserisci_e_cancella() {
        let mut t: Text<16> = Text::new();
        for b in b"ciao" {
            t.insert(*b).unwrap();
        }
        t.left();
        t.insert(b'X').unwrap();
        assert_eq!(t.bytes(), b"ciaXo");
        assert!(t.backspace());
        assert_eq!(t.bytes(), b"ciao");
        assert!(t.delete());
        assert_eq!(t.bytes(), b"cia");
        assert!(!t.delete());
    }

    #[test]
    fn cr_non_si_inserisce() {
        let mut t: Text<4> = Text::new();
        assert_eq!(t.insert(b'\r'), Err(TextError::BadChar));
    }

    #[test]
    fn pieno_non_cambia() {
        let mut t: Text<2> = Text::new();
        t.load(b"ab").unwrap();
        assert_eq!(t.insert(b'c'), Err(TextError::Full));
        assert_eq!(t.bytes(), b"ab");
    }

    #[test]
    fn sinistra_destra_ai_bordi() {
        let mut t: Text<8> = Text::new();
        t.load(b"ab").unwrap();
        assert!(!t.left());
        assert!(t.right() && t.right());
        assert!(!t.right());
    }

    #[test]
    fn su_giu_mantiene_colonna() {
        let mut t: Text<32> = Text::new();
        t.load(b"abcdef\nxy\nghijkl").unwrap();
        t.cur = 5; // riga 0, colonna 5
        assert!(t.down());
        assert_eq!(t.line_col(), (1, 2)); // riga corta: fine riga
        assert!(t.down());
        assert_eq!(t.line_col(), (2, 2)); // colonna ripresa
        assert!(!t.down());
        assert!(t.up() && t.up());
        assert_eq!(t.line_col(), (0, 2));
        assert!(!t.up());
    }

    #[test]
    fn righe() {
        let mut t: Text<32> = Text::new();
        t.load(b"uno\n\ntre").unwrap();
        assert_eq!(t.lines(), 3);
        assert_eq!(t.line(0), Some(&b"uno"[..]));
        assert_eq!(t.line(1), Some(&b""[..]));
        assert_eq!(t.line(2), Some(&b"tre"[..]));
        assert_eq!(t.line(3), None);
    }
}
