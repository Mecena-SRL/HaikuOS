//! Pila di finestre: l'ultima è in cima (attiva). Massimo `MAX_WIN`, senza heap.
use crate::layout::{close_button, Rect};

pub const MAX_WIN: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Win {
    pub app: u8,
    pub title: &'static str,
    pub frame: Rect,
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

    /// Finestra attiva (in cima).
    pub fn top(&self) -> Option<Win> {
        self.get(self.n.wrapping_sub(1))
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
        let y = (crate::layout::BAR_H + (screen_h - crate::layout::BAR_H - h) / 2).max(crate::layout::BAR_H);
        self.wins[self.n] = Some(Win { app, title, frame: Rect::new(x, y, w, h) });
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

    /// Passa alla finestra sotto (la più in basso diventa attiva).
    pub fn cycle(&mut self) {
        if self.n > 1 {
            let first = self.wins[0];
            for j in 0..self.n - 1 {
                self.wins[j] = self.wins[j + 1];
            }
            self.wins[self.n - 1] = first;
        }
    }

    /// Finestra più in alto che contiene il punto.
    pub fn hit(&self, x: i32, y: i32) -> Option<usize> {
        (0..self.n).rev().find(|&i| self.wins[i].is_some_and(|w| w.frame.contains(x, y)))
    }

    pub fn close_hit(&self, i: usize, x: i32, y: i32) -> bool {
        self.get(i).is_some_and(|w| close_button(w.frame).contains(x, y))
    }
}
