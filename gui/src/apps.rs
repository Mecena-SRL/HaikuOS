//! App di sistema con stato: calcolatrice, terminale (sysapps::shell), file.
//! Browser ed editor restano con testo statico (fase successiva).
use sysapps::{shell, Store};

pub const LINE_H: i32 = crate::layout::FONT_H + 1;
/// Righe di testo in una finestra alta 80 px (corpo 60 px, passo 6 px).
pub const TEXT_ROWS: usize = 10;
/// Temi disponibili (vedi `Shell::next_theme`); i nomi sono per IMPOST.
/// Id dello slot app IDE (finestra vuota finché il thread IDE non la collega).
pub const IDE: u8 = 9;
pub const THEMES: u8 = 3;
pub const THEME_NAMES: [&str; 3] = ["BLU", "VERDE", "GRAFITE"];
/// Byte per riga del terminale e righe di storico.
pub const TL: usize = 32;
pub const HN: usize = 8;

/// File di sistema, letti dal terminale e dall'app FILE.
pub const FILES: &[(&str, &str)] = &[
    ("leggimi.txt", "Benvenuto in HaikuOS"),
    ("note.txt", "Prima nota"),
    ("home.txt", "# HAIKUOS\n=> leggimi.txt LEGGIMI\n=> script.txt SCRIPT\nPagine offline"),
    ("script.txt", "echo ciao\necho due"),
];
/// Pagina iniziale del browser e script caricato nell'editor.
pub const HOME: &str = "home.txt";
pub const SCRIPT: &str = "script.txt";

pub struct Disk;

impl Store for Disk {
    fn file(&self, name: &str) -> Option<&'static str> {
        FILES.iter().find(|(n, _)| *n == name).map(|(_, b)| *b)
    }
    fn names(&self, out: &mut dyn FnMut(&str)) {
        for (n, _) in FILES {
            out(n);
        }
    }
}


/// Numero in decimale, senza heap.
pub fn num_str(v: i64, buf: &mut [u8; 21]) -> &str {
    let mut n = v.unsigned_abs();
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    if v < 0 {
        i -= 1;
        buf[i] = b'-';
    }
    core::str::from_utf8(&buf[i..]).unwrap_or("")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Calc {
    acc: i64,
    op: u8,
    cur: i64,
    typing: bool,
    err: bool,
}

impl Calc {
    pub const fn new() -> Calc {
        Calc { acc: 0, op: 0, cur: 0, typing: false, err: false }
    }

    /// Cifre, + - * /, = o Invio, Backspace, C per azzerare.
    pub fn key(&mut self, c: u8) {
        if self.err && !matches!(c, b'c' | b'C') {
            return;
        }
        match c {
            b'0'..=b'9' => {
                if !self.typing {
                    self.cur = 0;
                    self.typing = true;
                }
                if self.cur < 100_000_000_000 {
                    self.cur = self.cur * 10 + (c - b'0') as i64;
                }
            }
            b'+' | b'-' | b'*' | b'/' => {
                self.step();
                self.op = c;
                self.cur = self.acc;
                self.typing = false;
            }
            b'=' | 0x0D => {
                self.step();
                self.op = 0;
                self.cur = self.acc;
                self.typing = false;
            }
            0x08 if self.typing => self.cur /= 10,
            b'c' | b'C' => *self = Calc::new(),
            _ => {}
        }
    }

    /// Applica l'operazione in sospeso con il numero digitato.
    fn step(&mut self) {
        if !self.typing {
            return;
        }
        self.acc = match self.op {
            0 => self.cur,
            b'+' => self.acc.saturating_add(self.cur),
            b'-' => self.acc.saturating_sub(self.cur),
            b'*' => self.acc.saturating_mul(self.cur),
            _ => {
                if self.cur == 0 {
                    self.err = true;
                    return;
                }
                self.acc / self.cur
            }
        };
    }

    /// Testo del display.
    pub fn show<'a>(&self, buf: &'a mut [u8; 21]) -> &'a str {
        if self.err {
            "ERRORE"
        } else {
            num_str(self.cur, buf)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Term {
    pub line: [u8; TL],
    pub len: usize,
    pub hist: [[u8; TL]; HN],
    pub hl: [usize; HN],
    pub hn: usize,
}

impl Term {
    pub const fn new() -> Term {
        Term { line: [0; TL], len: 0, hist: [[0; TL]; HN], hl: [0; HN], hn: 0 }
    }

    /// Un carattere stampabile o Backspace.
    pub fn type_char(&mut self, c: u8) {
        match c {
            0x08 => self.len = self.len.saturating_sub(1),
            b' '..=b'~' if self.len < TL => {
                self.line[self.len] = c;
                self.len += 1;
            }
            _ => {}
        }
    }

    /// Riga di storico (troncata a TL byte). Quando è piena scorre di una riga.
    pub fn add(&mut self, s: &[u8]) {
        if self.hn == HN {
            self.hist.copy_within(1..HN, 0);
            self.hl.copy_within(1..HN, 0);
            self.hn -= 1;
        }
        let m = s.len().min(TL);
        self.hist[self.hn][..m].copy_from_slice(&s[..m]);
        self.hl[self.hn] = m;
        self.hn += 1;
    }

    /// Invio: mostra il comando, lo esegue con sysapps e raccoglie l'uscita.
    pub fn enter(&mut self) {
        let n = self.len;
        let mut cmd = [0u8; TL];
        cmd[..n].copy_from_slice(&self.line[..n]);
        self.len = 0;
        let mut row = [0u8; TL];
        row[..2].copy_from_slice(b": ");  // prompt: il font 3×5 non ha $
        let k = n.min(TL - 2);
        row[2..2 + k].copy_from_slice(&cmd[..k]);
        self.add(&row[..2 + k]);
        let text = core::str::from_utf8(&cmd[..n]).unwrap_or("");
        let mut out = |s: &str| self.add(s.as_bytes());
        if shell::exec(text, &Disk, &mut out, 0).is_err() {
            self.add(b"ERRORE");
        }
    }
}

/// App FILE: file selezionato e, se aperto, il suo contenuto.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileView {
    pub sel: usize,
    pub open: bool,
}

impl FileView {
    pub const fn new() -> FileView {
        FileView { sel: 0, open: false }
    }
}

/// Id delle app documenti e scacchi (kernel x86).
pub const DOC: u8 = 10;
pub const CHESS: u8 = 11;

/// Scacchi: cursore e casa selezionata (64 = nessuna). La partita è in `Shell::board`.
pub struct Chess {
    pub cur: u8,
    pub sel: u8,
}

impl Chess {
    pub const fn new() -> Chess {
        Chess { cur: 12, sel: 64 }
    }
}
