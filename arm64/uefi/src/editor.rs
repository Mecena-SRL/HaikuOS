//! Editor di testo: un buffer di byte con a capo. Solo `core`, testabile sull'host.

pub const CAP: usize = 256;

pub struct Editor {
    buf: [u8; CAP],
    len: usize,
}

impl Editor {
    pub const fn new() -> Self {
        Editor { buf: [0; CAP], len: 0 }
    }

    /// Un tasto: caratteri stampabili, Invio = a capo, Backspace cancella. Ritorna false se il buffer è pieno.
    pub fn key(&mut self, ch: u8) -> bool {
        match ch {
            0x08 | 0x7F => {
                self.len = self.len.saturating_sub(1);
                true
            }
            b'\r' | b'\n' | 0x20..=0x7E => {
                if self.len < CAP {
                    self.buf[self.len] = if ch == b'\r' { b'\n' } else { ch };
                    self.len += 1;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn text(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }

    /// Ultime `out.len()` righe (dopo gli a capo), ciascuna limitata agli ultimi `width` caratteri.
    pub fn last_lines<'a>(&'a self, out: &mut [&'a str], width: usize) -> usize {
        let text = self.text();
        let mut lines: [&str; 64] = [""; 64];
        let mut n = 0;
        for l in text.split('\n') {
            lines[n % 64] = l;
            n += 1;
        }
        let shown = n.min(out.len());
        for k in 0..shown {
            let l = lines[(n - shown + k) % 64];
            out[k] = &l[l.len().saturating_sub(width)..];
        }
        shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrive_e_cancella() {
        let mut e = Editor::new();
        for b in "ciao\rmondo".bytes() {
            e.key(b);
        }
        assert_eq!(e.text(), "ciao\nmondo");
        e.key(0x08);
        assert_eq!(e.text(), "ciao\nmond");
    }

    #[test]
    fn ultime_righe() {
        let mut e = Editor::new();
        for b in "uno\rdue\rtre".bytes() {
            e.key(b);
        }
        let mut out = [""; 2];
        let n = e.last_lines(&mut out, 10);
        assert_eq!(n, 2);
        assert_eq!(out, ["due", "tre"]);
    }
}
