//! Lettore di testo a pagine: a capo per parola, pagine di `rows` righe da `cols` caratteri.
//! Conta i caratteri (non i byte), quindi gli accenti contano uno. Regole: docs/apps-media.md.

/// Va a capo per parola. Una parola più lunga di `cols` viene spezzata.
/// Ogni riga vuota del testo resta come riga vuota.
pub fn wrap(text: &str, cols: usize) -> Vec<String> {
    let cols = cols.max(1);
    let mut lines = Vec::new();
    for para in text.lines() {
        let mut line = String::new();
        let mut n = 0;
        for word in para.split_whitespace() {
            let wl = word.chars().count();
            if n > 0 && n + 1 + wl <= cols {
                line.push(' ');
                line.push_str(word);
                n += 1 + wl;
                continue;
            }
            if n > 0 {
                lines.push(std::mem::take(&mut line));
            }
            n = 0;
            for ch in word.chars() {
                if n == cols {
                    lines.push(std::mem::take(&mut line));
                    n = 0;
                }
                line.push(ch);
                n += 1;
            }
        }
        lines.push(line);
    }
    lines
}

pub struct Reader {
    pages: Vec<Vec<String>>,
    page: usize,
}

impl Reader {
    pub fn new(text: &str, cols: usize, rows: usize) -> Self {
        let lines = wrap(text, cols);
        let pages = if lines.is_empty() {
            vec![Vec::new()]
        } else {
            lines.chunks(rows.max(1)).map(|c| c.to_vec()).collect()
        };
        Reader { pages, page: 0 }
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Pagina corrente, da 0.
    pub fn page(&self) -> usize {
        self.page
    }

    pub fn lines(&self) -> &[String] {
        &self.pages[self.page]
    }

    /// Pagina successiva. `false` se si è già all'ultima.
    pub fn next(&mut self) -> bool {
        if self.page + 1 < self.pages.len() {
            self.page += 1;
            true
        } else {
            false
        }
    }

    /// Pagina precedente. `false` se si è già alla prima.
    pub fn prev(&mut self) -> bool {
        if self.page > 0 {
            self.page -= 1;
            true
        } else {
            false
        }
    }
}
