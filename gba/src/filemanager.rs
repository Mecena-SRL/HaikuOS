//! File manager (prototipo): elenco della directory corrente, cursore e navigazione.
//! Logica pura, senza disegno: il compositore disegna `visible()`.
//! Usa solo `fs::Fs` in sola lettura. Nessun ordinamento: l'ordine è quello dello store (da verificare).
#![allow(dead_code)]

use crate::fs::{parent_of, Fs, FILES, NAME_MAX};

/// Righe visibili nella finestra. Da verificare contro l'altezza della superficie.
pub const ROWS: usize = 8;

pub struct FileManager {
    dir: [u8; NAME_MAX],
    dir_len: usize,
    items: [usize; FILES],
    count: usize,
    cursor: usize,
    top: usize,
}

impl FileManager {
    /// Parte dalla radice.
    pub const fn new() -> Self {
        Self { dir: [0; NAME_MAX], dir_len: 0, items: [0; FILES], count: 0, cursor: 0, top: 0 }
    }

    /// Percorso della directory corrente ("" = radice).
    pub fn dir(&self) -> &[u8] {
        &self.dir[..self.dir_len]
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Id delle voci visibili nella finestra, dall'alto in basso.
    pub fn visible(&self) -> &[usize] {
        let end = (self.top + ROWS).min(self.count);
        &self.items[self.top..end]
    }

    /// Rilegge la directory corrente dallo store. Chiamare dopo ogni modifica dello store.
    pub fn refresh(&mut self, fs: &Fs) {
        self.count = fs.list(&self.dir[..self.dir_len], &mut self.items);
        if self.cursor >= self.count {
            self.cursor = self.count.saturating_sub(1);
        }
        self.keep_visible();
    }

    pub fn down(&mut self) {
        if self.cursor + 1 < self.count {
            self.cursor += 1;
            self.keep_visible();
        }
    }

    pub fn up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.keep_visible();
        }
    }

    /// Apre la voce sotto il cursore. Una directory cambia la vista e ritorna `None`.
    /// Un file ritorna il suo id: aprirlo spetta al chiamante.
    pub fn enter(&mut self, fs: &Fs) -> Option<usize> {
        if self.cursor >= self.count {
            return None;
        }
        let id = self.items[self.cursor];
        if !fs.is_dir(id) {
            return Some(id);
        }
        let (name, _) = fs.info(id)?;
        self.dir[..name.len()].copy_from_slice(name);
        self.dir_len = name.len();
        self.reset_view();
        self.refresh(fs);
        None
    }

    /// Sale alla directory padre. Alla radice non fa nulla.
    pub fn back(&mut self, fs: &Fs) {
        if self.dir_len == 0 {
            return;
        }
        let parent = parent_of(&self.dir[..self.dir_len]).map_or(0, |p| p.len());
        self.dir_len = parent;
        self.reset_view();
        self.refresh(fs);
    }

    fn reset_view(&mut self) {
        self.cursor = 0;
        self.top = 0;
    }

    fn keep_visible(&mut self) {
        if self.cursor < self.top {
            self.top = self.cursor;
        } else if self.cursor >= self.top + ROWS {
            self.top = self.cursor + 1 - ROWS;
        }
    }
}
