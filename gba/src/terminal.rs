//! Terminale del desktop: `ls` e `cd` scelti dal menu con il keypad. Stato puro, senza hardware.
//! Il menu è: 0 = LS, 1 = CD .., poi CD per ogni sottodirectory di cwd (max 6).

use crate::fs::{parent_of, Fs, FILES, NAME_MAX};

pub const LINES: usize = 4;
pub const LINE_MAX: usize = 14;
pub const LABEL_MAX: usize = 24;
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

    fn set_line(&mut self, s: &[u8]) {
        if self.out_n < LINES {
            let k = self.out_n;
            let mut l = 0;
            push(&mut self.out[k], &mut l, s);
            self.out_len[k] = l;
            self.out_n += 1;
        }
    }

    /// Esegue la voce `i` del menu. `ls` riempie l'output; `cd` cambia cwd e pulisce l'output.
    pub fn run(&mut self, fs: &Fs, i: usize) {
        let cmd = match self.cmd(fs, i) {
            Some(c) => c,
            None => return,
        };
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

    pub fn lines(&self) -> usize {
        self.out_n
    }

    pub fn line(&self, k: usize) -> &[u8] {
        &self.out[k][..self.out_len[k]]
    }
}
