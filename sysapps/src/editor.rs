//! Editor di script: buffer a righe fisse (nessun heap).
//! Dimensioni (LINES, WIDTH) scelte dal chiamante. Su GBA da verificare contro la RAM.

use crate::shell::{self, ShellError};
use crate::Store;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditError {
    Full,
    /// Solo ASCII stampabile.
    Invalid,
}

pub struct Editor<const LINES: usize, const WIDTH: usize> {
    rows: [[u8; WIDTH]; LINES],
    lens: [usize; LINES],
    count: usize,
    row: usize,
    col: usize,
}

impl<const LINES: usize, const WIDTH: usize> Editor<LINES, WIDTH> {
    pub const fn new() -> Self {
        Self { rows: [[0; WIDTH]; LINES], lens: [0; LINES], count: 1, row: 0, col: 0 }
    }

    /// Sostituisce il contenuto con `text`.
    pub fn load(&mut self, text: &str) -> Result<(), EditError> {
        *self = Self::new();
        for (i, line) in text.lines().enumerate() {
            if i > 0 {
                self.newline()?;
            }
            for &b in line.as_bytes() {
                self.insert(b)?;
            }
        }
        self.row = 0;
        self.col = 0;
        Ok(())
    }

    pub fn lines(&self) -> usize {
        self.count
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.row, self.col)
    }

    pub fn line(&self, i: usize) -> &str {
        if i >= self.count {
            return "";
        }
        core::str::from_utf8(&self.rows[i][..self.lens[i]]).unwrap_or("")
    }

    pub fn insert(&mut self, b: u8) -> Result<(), EditError> {
        if !(0x20..0x7f).contains(&b) {
            return Err(EditError::Invalid);
        }
        let len = self.lens[self.row];
        if len == WIDTH {
            return Err(EditError::Full);
        }
        self.rows[self.row].copy_within(self.col..len, self.col + 1);
        self.rows[self.row][self.col] = b;
        self.lens[self.row] += 1;
        self.col += 1;
        Ok(())
    }

    pub fn newline(&mut self) -> Result<(), EditError> {
        if self.count == LINES {
            return Err(EditError::Full);
        }
        let len = self.lens[self.row];
        let mut tail = [0u8; WIDTH];
        let n = len - self.col;
        tail[..n].copy_from_slice(&self.rows[self.row][self.col..len]);
        for i in (self.row + 1..self.count).rev() {
            self.rows[i + 1] = self.rows[i];
            self.lens[i + 1] = self.lens[i];
        }
        self.rows[self.row + 1] = tail;
        self.lens[self.row + 1] = n;
        self.lens[self.row] = self.col;
        self.count += 1;
        self.row += 1;
        self.col = 0;
        Ok(())
    }

    /// Cancella il carattere prima del cursore; a inizio riga unisce alla riga precedente.
    pub fn backspace(&mut self) -> Result<(), EditError> {
        if self.col > 0 {
            let len = self.lens[self.row];
            self.rows[self.row].copy_within(self.col..len, self.col - 1);
            self.lens[self.row] -= 1;
            self.col -= 1;
        } else if self.row > 0 {
            let prev = self.lens[self.row - 1];
            let cur = self.lens[self.row];
            if prev + cur > WIDTH {
                return Err(EditError::Full);
            }
            let cur_row = self.rows[self.row];
            self.rows[self.row - 1][prev..prev + cur].copy_from_slice(&cur_row[..cur]);
            self.lens[self.row - 1] = prev + cur;
            for i in self.row..self.count - 1 {
                self.rows[i] = self.rows[i + 1];
                self.lens[i] = self.lens[i + 1];
            }
            self.count -= 1;
            self.row -= 1;
            self.col = prev;
        }
        Ok(())
    }

    pub fn left(&mut self) {
        if self.col > 0 {
            self.col -= 1;
        } else if self.row > 0 {
            self.row -= 1;
            self.col = self.lens[self.row];
        }
    }

    pub fn right(&mut self) {
        if self.col < self.lens[self.row] {
            self.col += 1;
        } else if self.row + 1 < self.count {
            self.row += 1;
            self.col = 0;
        }
    }

    pub fn up(&mut self) {
        if self.row > 0 {
            self.row -= 1;
            self.col = self.col.min(self.lens[self.row]);
        }
    }

    pub fn down(&mut self) {
        if self.row + 1 < self.count {
            self.row += 1;
            self.col = self.col.min(self.lens[self.row]);
        }
    }

    /// Esegue il buffer riga per riga. Errore: (numero riga da 0, motivo).
    pub fn run(&self, store: &dyn Store, out: &mut dyn FnMut(&str)) -> Result<(), (usize, ShellError)> {
        run_lines(store, (0..self.count).map(|i| self.line(i)), out)
    }
}

/// Esegue righe di script. Le righe vengono eseguite come dentro uno script (depth 1).
pub fn run_lines<'a>(
    store: &dyn Store,
    lines: impl Iterator<Item = &'a str>,
    out: &mut dyn FnMut(&str),
) -> Result<(), (usize, ShellError)> {
    for (n, line) in lines.enumerate() {
        shell::exec(line, store, out, 1).map_err(|e| (n, e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Files;

    static STORE: Files = Files(&[]);

    fn text<const L: usize, const W: usize>(e: &Editor<L, W>) -> std::string::String {
        (0..e.lines()).map(|i| e.line(i)).collect::<std::vec::Vec<_>>().join("\n")
    }

    #[test]
    fn type_and_newline() {
        let mut e: Editor<4, 16> = Editor::new();
        for &b in b"echo a" {
            e.insert(b).unwrap();
        }
        e.newline().unwrap();
        for &b in b"apps" {
            e.insert(b).unwrap();
        }
        assert_eq!(text(&e), "echo a\napps");
        assert_eq!(e.cursor(), (1, 4));
    }

    #[test]
    fn insert_in_middle_and_backspace_merge() {
        let mut e: Editor<4, 8> = Editor::new();
        e.load("ab\ncd").unwrap();
        e.right();
        e.insert(b'X').unwrap();
        assert_eq!(text(&e), "aXb\ncd");
        e.down();
        e.left();
        e.left();
        e.backspace().unwrap();
        assert_eq!(text(&e), "aXbcd");
        assert_eq!(e.cursor(), (0, 3));
    }

    #[test]
    fn limits_and_invalid_bytes() {
        let mut e: Editor<2, 2> = Editor::new();
        e.insert(b'a').unwrap();
        e.insert(b'b').unwrap();
        assert_eq!(e.insert(b'c'), Err(EditError::Full));
        e.newline().unwrap();
        assert_eq!(e.newline(), Err(EditError::Full));
        assert_eq!(e.insert(b'\n'), Err(EditError::Invalid));
    }

    #[test]
    fn run_buffer_reports_error_line() {
        let mut e: Editor<4, 32> = Editor::new();
        e.load("echo ok\nfoo").unwrap();
        let mut out = std::vec::Vec::new();
        let r = e.run(&STORE, &mut |s: &str| out.push(s.to_string()));
        assert_eq!(out, ["ok"]);
        assert_eq!(r, Err((1, ShellError::Unknown)));
    }
}
