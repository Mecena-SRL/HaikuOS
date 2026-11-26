//! Nucleo dell'IDE, senza interfaccia: buffer di testo con annulla.
//! La GUI (thread desktop) lo pilota con: insert, newline, backspace, undo.

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Buffer {
    lines: Vec<String>,
    row: usize,
    col: usize,
    undo: Vec<(Vec<String>, usize, usize)>,
}

impl Buffer {
    pub fn new(text: &str) -> Self {
        let mut lines: Vec<String> = text.split('\n').map(String::from).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Buffer { lines, ..Default::default() }
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.row, self.col)
    }

    /// Salva lo stato prima di una modifica (per annullare).
    fn snap(&mut self) {
        self.undo.push((self.lines.clone(), self.row, self.col));
    }

    pub fn insert(&mut self, c: char) {
        self.snap();
        let line = &mut self.lines[self.row];
        let at = line.char_indices().nth(self.col).map_or(line.len(), |(i, _)| i);
        line.insert(at, c);
        self.col += 1;
    }

    pub fn newline(&mut self) {
        self.snap();
        let line = &mut self.lines[self.row];
        let at = line.char_indices().nth(self.col).map_or(line.len(), |(i, _)| i);
        let rest = line.split_off(at);
        self.row += 1;
        self.lines.insert(self.row, rest);
        self.col = 0;
    }

    /// Cancella il carattere prima del cursore, o unisce con la riga precedente.
    pub fn backspace(&mut self) {
        if self.row == 0 && self.col == 0 {
            return;
        }
        self.snap();
        if self.col > 0 {
            let line = &mut self.lines[self.row];
            let (i, _) = line.char_indices().nth(self.col - 1).unwrap();
            line.remove(i);
            self.col -= 1;
        } else {
            let cur = self.lines.remove(self.row);
            self.row -= 1;
            self.col = self.lines[self.row].chars().count();
            self.lines[self.row].push_str(&cur);
        }
    }

    pub fn undo(&mut self) -> bool {
        match self.undo.pop() {
            Some((lines, row, col)) => {
                self.lines = lines;
                self.row = row;
                self.col = col;
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_and_newline() {
        let mut b = Buffer::new("");
        for c in "ciao".chars() {
            b.insert(c);
        }
        b.newline();
        b.insert('x');
        assert_eq!(b.text(), "ciao\nx");
        assert_eq!(b.cursor(), (1, 1));
    }

    #[test]
    fn backspace_joins_lines() {
        let mut b = Buffer::new("ab\ncd");
        b.row = 1;
        b.col = 0;
        b.backspace();
        assert_eq!(b.text(), "abcd");
        assert_eq!(b.cursor(), (0, 2));
    }

    #[test]
    fn undo_restores_previous_state() {
        let mut b = Buffer::new("");
        b.insert('a');
        b.insert('b');
        assert!(b.undo());
        assert_eq!(b.text(), "a");
        assert!(b.undo());
        assert_eq!(b.text(), "");
        assert!(!b.undo());
    }

    #[test]
    fn handles_utf8() {
        let mut b = Buffer::new("è");
        b.col = 1;
        b.insert('x');
        b.backspace();
        b.backspace();
        assert_eq!(b.text(), "");
    }
}
