//! Terminale del desktop: `ls` e `cd` dal menu col keypad, o digitati lettera per lettera.
//! Menu: 0 = LS, 1 = CD .., poi CD per ogni sottodirectory di cwd (max 6).
//! Digitazione (SELECT): su/giù scelgono il carattere, A lo aggiunge, B cancella, sinistra esegue.
//! Comandi digitati: LS, UP (= cd ..), CD nome. Il font ha solo alcune lettere: vedi ALPHA.

use crate::fs::{parent_of, Fs, FILES, NAME_MAX};

pub const LINES: usize = 4;
pub const LINE_MAX: usize = 14;
pub const LABEL_MAX: usize = 24;
pub const INPUT_MAX: usize = 16;
/// Caratteri scelti in digitazione: solo quelli che il font disegna (vedi font.rs).
pub const ALPHA: &[u8] = b" ABCDEFGHIKLMNORSTUW012";
const DIRS: usize = 6;

#[derive(Clone, Copy, PartialEq)]
enum Cmd {
    Ls,
    Up,
    Cd(usize),
}

pub struct Terminal {
    cwd: [u8; NAME_MAX],
    cwd_len: usize,
    pub sel: usize,
    pub typing: bool,
    pub pick: usize,
    input: [u8; INPUT_MAX],
    input_len: usize,
    out: [[u8; LINE_MAX]; LINES],
    out_len: [usize; LINES],
    out_n: usize,
}

fn push(buf: &mut [u8], n: &mut usize, s: &[u8]) {
    for &c in s {
        if *n < buf.len() {
            buf[*n] = c;
            *n += 1;
        }
    }
}

/// Ultimo componente del percorso: "a/b" -> "b".
fn last(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|&c| c == b'/') {
        Some(i) => &name[i + 1..],
        None => name,
    }
}

impl Terminal {
    pub const fn new() -> Self {
        Self {
            cwd: [0; NAME_MAX],
            cwd_len: 0,
            sel: 0,
            typing: false,
            pick: 0,
            input: [0; INPUT_MAX],
            input_len: 0,
            out: [[0; LINE_MAX]; LINES],
            out_len: [0; LINES],
            out_n: 0,
        }
    }

    pub fn cwd(&self) -> &[u8] {
        &self.cwd[..self.cwd_len]
    }

    /// Sottodirectory di cwd, al massimo DIRS. Ritorna quante ne ha scritte in `out`.
    fn subdirs(&self, fs: &Fs, out: &mut [usize; DIRS]) -> usize {
        let mut ids = [0usize; FILES];
        let n = fs.list(self.cwd(), &mut ids);
        let mut k = 0;
        for &id in &ids[..n] {
            if k < DIRS && fs.is_dir(id) {
                out[k] = id;
                k += 1;
            }
        }
        k
    }

    pub fn menu_len(&self, fs: &Fs) -> usize {
        let mut d = [0usize; DIRS];
        2 + self.subdirs(fs, &mut d)
    }

    fn cmd(&self, fs: &Fs, i: usize) -> Option<Cmd> {
        match i {
            0 => Some(Cmd::Ls),
            1 => Some(Cmd::Up),
            _ => {
                let mut d = [0usize; DIRS];
                let n = self.subdirs(fs, &mut d);
                if i - 2 < n {
                    Some(Cmd::Cd(d[i - 2]))
                } else {
                    None
                }
            }
        }
    }

    /// Etichetta della voce `i` ("LS", "CD ..", "CD nome"). Ritorna la lunghezza scritta in `buf`.
    pub fn label(&self, fs: &Fs, i: usize, buf: &mut [u8; LABEL_MAX]) -> usize {
        let mut n = 0;
        match self.cmd(fs, i) {
            Some(Cmd::Ls) => push(buf, &mut n, b"LS"),
            Some(Cmd::Up) => push(buf, &mut n, b"CD .."),
            Some(Cmd::Cd(id)) => {
                push(buf, &mut n, b"CD ");
                if let Some((name, _)) = fs.info(id) {
                    push(buf, &mut n, last(name));
                }
            }
            None => {}
        }
        n
    }

    /// Muove la selezione nel menu, con ritorno da fine a inizio.
    pub fn step_sel(&mut self, fs: &Fs, delta: isize) {
        let m = self.menu_len(fs) as isize;
        self.sel = (self.sel as isize + delta).rem_euclid(m) as usize;
    }

    pub fn toggle_typing(&mut self) {
        self.typing = !self.typing;
    }

    pub fn pick_step(&mut self, delta: isize) {
        self.pick = (self.pick as isize + delta).rem_euclid(ALPHA.len() as isize) as usize;
    }

    /// Aggiunge un byte alla riga digitata (se c'è spazio).
    pub fn type_byte(&mut self, c: u8) {
        if self.input_len < INPUT_MAX {
            self.input[self.input_len] = c;
            self.input_len += 1;
        }
    }

    /// Aggiunge il carattere scelto con su/giù.
    pub fn type_pick(&mut self) {
        self.type_byte(ALPHA[self.pick]);
    }

    pub fn backspace(&mut self) {
        self.input_len = self.input_len.saturating_sub(1);
    }

    pub fn input(&self) -> &[u8] {
        &self.input[..self.input_len]
    }

    /// Esegue la riga digitata e la svuota. LS, UP, CD nome (senza distinguere maiuscole).
    pub fn run_input(&mut self, fs: &Fs) {
        let line = self.input; // copia: la riga non è più in uso dopo il comando
        let line = &line[..self.input_len];
        let line = trim(line);
        self.input_len = 0;
        if line.eq_ignore_ascii_case(b"LS") {
            self.run_cmd(fs, Cmd::Ls);
        } else if line.eq_ignore_ascii_case(b"UP") {
            self.run_cmd(fs, Cmd::Up);
        } else if line.len() > 3 && line[..3].eq_ignore_ascii_case(b"CD ") {
            match self.find_child(fs, trim(&line[3..])) {
                Some(id) => self.run_cmd(fs, Cmd::Cd(id)),
                None => {
                    self.out_n = 0;
                    self.set_line(b"NON TROVATO");
                }
            }
        } else {
            self.out_n = 0;
            self.set_line(b"ERRORE");
        }
    }

    /// Sottodirectory di cwd con questo nome (ultimo componente, senza distinguere maiuscole).
    fn find_child(&self, fs: &Fs, want: &[u8]) -> Option<usize> {
        let mut ids = [0usize; FILES];
        let n = fs.list(self.cwd(), &mut ids);
        for &id in &ids[..n] {
            if let Some((name, _)) = fs.info(id) {
                if fs.is_dir(id) && last(name).eq_ignore_ascii_case(want) {
                    return Some(id);
                }
            }
        }
        None
    }

    fn set_line(&mut self, s: &[u8]) {
        if self.out_n < LINES {
            let k = self.out_n;
            let mut l = 0;
            push(&mut self.out[k], &mut l, s);
            self.out_len[k] = l;
            self.out_n += 1;
        }
    }

    /// Esegue la voce `i` del menu.
    pub fn run(&mut self, fs: &Fs, i: usize) {
        if let Some(cmd) = self.cmd(fs, i) {
            self.run_cmd(fs, cmd);
        }
    }

    fn run_cmd(&mut self, fs: &Fs, cmd: Cmd) {
        self.out_n = 0;
        match cmd {
            Cmd::Ls => {
                let mut ids = [0usize; FILES];
                let n = fs.list(self.cwd(), &mut ids);
                for &id in &ids[..n] {
                    if let Some((name, _)) = fs.info(id) {
                        self.set_line(last(name));
                    }
                }
                if n == 0 {
                    self.set_line(b"VUOTO");
                }
            }
            Cmd::Up => {
                self.cwd_len = parent_of(self.cwd()).map_or(0, |p| p.len());
                self.sel = 0;
            }
            Cmd::Cd(id) => {
                if let Some((name, _)) = fs.info(id) {
                    let n = name.len().min(NAME_MAX);
                    self.cwd[..n].copy_from_slice(&name[..n]);
                    self.cwd_len = n;
                    self.sel = 0;
                }
            }
        }
    }

    /// Imposta la cartella corrente (es. quella scelta nel file manager). Se cambia, pulisce l'output.
    pub fn set_cwd(&mut self, path: &[u8]) {
        if path == self.cwd() {
            return; // stessa cartella: l'output resta com'è
        }
        let n = path.len().min(NAME_MAX);
        self.cwd[..n].copy_from_slice(&path[..n]);
        self.cwd_len = n;
        self.sel = 0;
        self.out_n = 0;
    }

    pub fn lines(&self) -> usize {
        self.out_n
    }

    pub fn line(&self, k: usize) -> &[u8] {
        &self.out[k][..self.out_len[k]]
    }
}

/// Toglie gli spazi iniziali e finali.
fn trim(s: &[u8]) -> &[u8] {
    let a = s.iter().position(|&c| c != b' ').unwrap_or(s.len());
    let b = s.iter().rposition(|&c| c != b' ').map_or(a, |i| i + 1);
    &s[a..b.max(a)]
}
