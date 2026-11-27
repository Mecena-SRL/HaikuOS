//! Terminale a riga: si scrive un comando e Invio lo esegue. Solo `core`, testabile sull'host.
//! Comandi: HELP, VER, ECHO testo, CLS, EXIT (chiude la finestra).

pub const MAX: usize = 40;

pub struct Term {
    line: [u8; MAX],
    len: usize,
    out: [u8; MAX],
    out_len: usize,
    /// Impostato da EXIT: la finestra va chiusa.
    pub exit: bool,
}

fn eq_ci(a: &[u8], b: &str) -> bool {
    a.len() == b.len() && a.iter().zip(b.bytes()).all(|(x, y)| x.to_ascii_uppercase() == y)
}

impl Term {
    pub const fn new() -> Self {
        Term { line: [0; MAX], len: 0, out: [0; MAX], out_len: 0, exit: false }
    }

    fn set_out(&mut self, s: &[u8]) {
        let n = s.len().min(MAX);
        self.out[..n].copy_from_slice(&s[..n]);
        self.out_len = n;
    }

    /// Un tasto. Ritorna true se è stato eseguito un comando (Invio).
    pub fn key(&mut self, ch: u8) -> bool {
        match ch {
            0x08 | 0x7F => {
                self.len = self.len.saturating_sub(1);
                false
            }
            b'\r' | b'\n' => {
                self.run();
                true
            }
            0x20..=0x7E => {
                if self.len < MAX {
                    self.line[self.len] = ch;
                    self.len += 1;
                }
                false
            }
            _ => false,
        }
    }

    fn run(&mut self) {
        let line = self.line;
        let s = &line[..self.len];
        let start = s.iter().position(|&c| c != b' ').unwrap_or(s.len());
        let s = &s[start..];
        let end = s.iter().position(|&c| c == b' ').unwrap_or(s.len());
        let (cmd, rest) = (&s[..end], &s[end..]);
        let arg = if rest.is_empty() { rest } else { &rest[1..] };
        if cmd.is_empty() {
            self.set_out(b"");
        } else if eq_ci(cmd, "HELP") {
            self.set_out(b"HELP VER ECHO CLS EXIT");
        } else if eq_ci(cmd, "VER") {
            self.set_out(b"HAIKUOS 0.1 ARM64");
        } else if eq_ci(cmd, "ECHO") {
            self.set_out(arg);
        } else if eq_ci(cmd, "CLS") {
            self.set_out(b"");
        } else if eq_ci(cmd, "EXIT") {
            self.exit = true;
            self.set_out(b"");
        } else {
            self.set_out(b"NON TROVATO");
        }
        self.len = 0;
    }

    /// Ultimo output (ASCII).
    pub fn output(&self) -> &str {
        core::str::from_utf8(&self.out[..self.out_len]).unwrap_or("")
    }

    /// Riga in scrittura (ASCII).
    pub fn line(&self) -> &str {
        core::str::from_utf8(&self.line[..self.len]).unwrap_or("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scrivi(t: &mut Term, s: &str) -> bool {
        let mut eseguito = false;
        for b in s.bytes() {
            eseguito = t.key(b);
        }
        eseguito
    }

    #[test]
    fn echo_e_ver() {
        let mut t = Term::new();
        assert!(scrivi(&mut t, "echo ciao mondo\r"));
        assert_eq!(t.output(), "ciao mondo");
        assert_eq!(t.line(), "");
        scrivi(&mut t, "Ver\r");
        assert_eq!(t.output(), "HAIKUOS 0.1 ARM64");
    }

    #[test]
    fn comando_sconosciuto_e_cancella() {
        let mut t = Term::new();
        scrivi(&mut t, "vzz\x08\x08\x08ver\r");
        assert_eq!(t.output(), "HAIKUOS 0.1 ARM64");
        scrivi(&mut t, "xyz\r");
        assert_eq!(t.output(), "NON TROVATO");
    }

    #[test]
    fn exit_e_cls() {
        let mut t = Term::new();
        scrivi(&mut t, "ver\rcls\r");
        assert_eq!(t.output(), "");
        assert!(!t.exit);
        scrivi(&mut t, "exit\r");
        assert!(t.exit);
    }

    #[test]
    fn riga_limitata() {
        let mut t = Term::new();
        for _ in 0..(MAX + 10) {
            t.key(b'a');
        }
        assert_eq!(t.line().len(), MAX);
    }
}
