//! Pila di finestre: l'ultima visibile è in cima (attiva). Massimo `MAX_WIN`, senza heap.
use crate::layout::{close_button, max_button, min_button, Rect, BAR_H};

pub const MAX_WIN: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Win {
    pub app: u8,
    pub title: &'static str,
    pub frame: Rect,
    /// Ridotta a icona: nascosta, resta nella barra.
    pub min: bool,
    /// Massimizzata: occupa tutta l'area sotto la barra.
    pub max: bool,
    back: Rect,
}

impl Default for Wm {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Wm {
    wins: [Option<Win>; MAX_WIN],
    n: usize,
}

impl Wm {
    pub const fn new() -> Wm {
        Wm { wins: [None; MAX_WIN], n: 0 }
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    pub fn get(&self, i: usize) -> Option<Win> {
        if i < self.n { self.wins[i] } else { None }
    }

    /// Indice della finestra attiva: l'ultima non ridotta a icona.
    pub fn active(&self) -> Option<usize> {
        (0..self.n).rev().find(|&i| self.get(i).is_some_and(|w| !w.min))
    }

    /// Finestra attiva.
    pub fn top(&self) -> Option<Win> {
        self.active().and_then(|i| self.get(i))
    }

    pub fn find_app(&self, app: u8) -> Option<usize> {
        (0..self.n).find(|&i| self.wins[i].map(|w| w.app) == Some(app))
    }

    /// Apre una finestra centrata. False se la pila è piena.
    pub fn open(&mut self, app: u8, title: &'static str, w: i32, h: i32, screen_w: i32, screen_h: i32) -> bool {
        if self.n == MAX_WIN {
            return false;
        }
        let x = ((screen_w - w) / 2).max(0);
        let y = (BAR_H + (screen_h - BAR_H - h) / 2).max(BAR_H);
        let frame = Rect::new(x, y, w, h);
        self.wins[self.n] = Some(Win { app, title, frame, min: false, max: false, back: frame });
        self.n += 1;
        true
    }

    /// Porta la finestra `i` in cima.
    pub fn raise(&mut self, i: usize) {
        if i + 1 >= self.n {
            return;
        }
        let w = self.wins[i];
        for j in i..self.n - 1 {
            self.wins[j] = self.wins[j + 1];
        }
        self.wins[self.n - 1] = w;
    }

    /// Chiude la finestra `i`; ritorna l'app.
    pub fn close(&mut self, i: usize) -> Option<u8> {
        let w = self.get(i)?;
        for j in i..self.n - 1 {
            self.wins[j] = self.wins[j + 1];
        }
        self.n -= 1;
        self.wins[self.n] = None;
        Some(w.app)
    }

    /// Alt-Tab: la finestra visibile più in basso diventa la attiva.
    pub fn cycle(&mut self) {
        if let Some(f) = (0..self.n).find(|&i| self.get(i).is_some_and(|w| !w.min)) {
            self.raise(f);
        }
    }

    /// Finestra visibile più in alto che contiene il punto.
    pub fn hit(&self, x: i32, y: i32) -> Option<usize> {
        (0..self.n).rev().find(|&i| self.get(i).is_some_and(|w| !w.min && w.frame.contains(x, y)))
    }

    pub fn minimize(&mut self, i: usize) {
        if let Some(w) = self.wins.get_mut(i).and_then(|w| w.as_mut()) {
            w.min = true;
        }
    }

    /// Ripristina una finestra ridotta a icona e la porta in cima.
    pub fn restore(&mut self, i: usize) {
        if let Some(w) = self.wins.get_mut(i).and_then(|w| w.as_mut()) {
            w.min = false;
        }
        self.raise(i);
    }

    /// Massimizza o ripristina la finestra `i` sull'area sotto la barra.
    pub fn toggle_max(&mut self, i: usize, screen_w: i32, screen_h: i32) {
        let Some(w) = self.wins.get_mut(i).and_then(|w| w.as_mut()) else { return };
        if w.max {
            w.frame = w.back;
            w.max = false;
        } else {
            w.back = w.frame;
            w.frame = Rect::new(0, BAR_H, screen_w, screen_h - BAR_H);
            w.max = true;
        }
    }

    /// Sposta la finestra `i` tenendola dentro lo schermo, sotto la barra. Niente se massimizzata.
    pub fn move_win(&mut self, i: usize, x: i32, y: i32, screen_w: i32, screen_h: i32) {
        let Some(w) = self.wins.get_mut(i).and_then(|w| w.as_mut()) else { return };
        if w.max {
            return;
        }
        w.frame.x = x.clamp(0, (screen_w - w.frame.w).max(0));
        w.frame.y = y.clamp(BAR_H, (screen_h - w.frame.h).max(BAR_H));
    }

    pub fn close_hit(&self, i: usize, x: i32, y: i32) -> bool {
        self.get(i).is_some_and(|w| close_button(w.frame).contains(x, y))
    }

    pub fn max_hit(&self, i: usize, x: i32, y: i32) -> bool {
        self.get(i).is_some_and(|w| max_button(w.frame).contains(x, y))
    }

    pub fn min_hit(&self, i: usize, x: i32, y: i32) -> bool {
        self.get(i).is_some_and(|w| min_button(w.frame).contains(x, y))
    }
}
