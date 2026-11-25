//! Calcolatrice: programma con superficie propria (SW x SH, quattro tinte), input dalla croce e da A.
//! Griglia 4x4 di tasti: la croce sposta il cursore, A preme il tasto. B è gestito dal desktop (chiude).
#![allow(dead_code)]

use crate::desktop::{T0, T1, T2};
use crate::font;

pub const SW: usize = 104;
pub const SH: usize = 74;
const CELL_W: usize = 26;
const CELL_H: usize = 14;
const GRID_Y: usize = 16;

pub const KEY_A: u16 = 1 << 0;
pub const KEY_RIGHT: u16 = 1 << 4;
pub const KEY_LEFT: u16 = 1 << 5;
pub const KEY_UP: u16 = 1 << 6;
pub const KEY_DOWN: u16 = 1 << 7;

/// Etichette dei 16 tasti, riga per riga.
pub const CELLS: [u8; 16] = [
    b'7', b'8', b'9', b'/', //
    b'4', b'5', b'6', b'*', //
    b'1', b'2', b'3', b'-', //
    b'C', b'0', b'=', b'+',
];

const MAX_DIGITS: i32 = 100_000_000;

pub struct Calc {
    cursor: usize,
    entry: i32,
    acc: i32,
    op: u8,
    fresh: bool,
    err: bool,
    prev: u16,
}

fn apply(a: i32, b: i32, op: u8) -> Option<i32> {
    match op {
        b'+' => a.checked_add(b),
        b'-' => a.checked_sub(b),
        b'*' => a.checked_mul(b),
        b'/' => {
            if b == 0 {
                None
            } else {
                a.checked_div(b)
            }
        }
        _ => Some(b),
    }
}

impl Calc {
    pub const fn new() -> Self {
        Self { cursor: 0, entry: 0, acc: 0, op: 0, fresh: false, err: false, prev: 0 }
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn value(&self) -> i32 {
        self.entry
    }

    pub fn is_err(&self) -> bool {
        self.err
    }

    /// Preme il tasto `cell` (0..16).
    pub fn press(&mut self, cell: usize) {
        let k = CELLS[cell % 16];
        match k {
            b'0'..=b'9' => self.digit((k - b'0') as i32),
            b'C' => *self = Calc { prev: self.prev, cursor: self.cursor, ..Calc::new() },
            b'=' => self.equals(),
            op => self.set_op(op),
        }
    }

    fn fail(&mut self) {
        self.err = true;
        self.entry = 0;
        self.acc = 0;
        self.op = 0;
        self.fresh = true;
    }

    fn digit(&mut self, d: i32) {
        if self.err {
            return;
        }
        if self.fresh {
            self.entry = 0;
            self.fresh = false;
        }
        if self.entry.abs() < MAX_DIGITS {
            self.entry = self.entry * 10 + d;
        }
    }

    fn set_op(&mut self, op: u8) {
        if self.err {
            return;
        }
        if self.op != 0 && !self.fresh {
            match apply(self.acc, self.entry, self.op) {
                Some(v) => {
                    self.acc = v;
                    self.entry = v;
                }
                None => return self.fail(),
            }
        } else if self.op == 0 {
            self.acc = self.entry;
        }
        self.op = op;
        self.fresh = true;
    }

    fn equals(&mut self) {
        if self.err || self.op == 0 {
            return;
        }
        match apply(self.acc, self.entry, self.op) {
            Some(v) => {
                self.entry = v;
                self.acc = v;
                self.op = 0;
                self.fresh = true;
            }
            None => self.fail(),
        }
    }

    /// Un passo di input. Ritorna true se bisogna ridisegnare.
    pub fn step(&mut self, keys: u16) -> bool {
        let pressed = keys & !self.prev;
        self.prev = keys;
        let row = self.cursor / 4;
        let col = self.cursor % 4;
        if pressed & KEY_RIGHT != 0 {
            self.cursor = row * 4 + (col + 1) % 4;
        }
        if pressed & KEY_LEFT != 0 {
            self.cursor = row * 4 + (col + 3) % 4;
        }
        if pressed & KEY_DOWN != 0 {
            self.cursor = (self.cursor + 4) % 16;
        }
        if pressed & KEY_UP != 0 {
            self.cursor = (self.cursor + 12) % 16;
        }
        if pressed & KEY_A != 0 {
            self.press(self.cursor);
        }
        pressed & (KEY_A | KEY_RIGHT | KEY_LEFT | KEY_UP | KEY_DOWN) != 0
    }

    /// Disegna la calcolatrice nella superficie `buf` (SW * SH pixel).
    pub fn draw(&self, buf: &mut [u16]) {
        for p in buf.iter_mut() {
            *p = T0;
        }
        // Display: numero o ERR, a destra.
        let mut text = [0u8; 12];
        let mut n = 0;
        if self.err {
            text[..3].copy_from_slice(b"ERR");
            n = 3;
        } else {
            let mut v = self.entry.unsigned_abs();
            let mut digits = [0u8; 10];
            let mut d = 0;
            loop {
                digits[d] = b'0' + (v % 10) as u8;
                v /= 10;
                d += 1;
                if v == 0 {
                    break;
                }
            }
            if self.entry < 0 {
                text[n] = b'-';
                n += 1;
            }
            while d > 0 {
                d -= 1;
                text[n] = digits[d];
                n += 1;
            }
        }
        let w = font::text_width(&text[..n]);
        let x = (SW - 3).saturating_sub(w);
        font::draw(buf, SW, SH, x, 4, &text[..n], T1);

        // Griglia: sfondo chiaro per i tasti, selezionato in bianco con testo scuro.
        for i in 0..16 {
            let cx = 2 + (i % 4) * CELL_W;
            let cy = GRID_Y + (i / 4) * CELL_H;
            let sel = i == self.cursor;
            let (bg, fg) = if sel { (T1, T0) } else { (T2, T0) };
            fill(buf, cx, cy, CELL_W - 2, CELL_H - 2, bg);
            let lbl = [CELLS[i]];
            let lx = cx + (CELL_W - 2 - font::GLYPH_W) / 2;
            let ly = cy + (CELL_H - 2 - font::GLYPH_H) / 2;
            font::draw(buf, SW, SH, lx, ly, &lbl, fg);
        }
    }
}

fn fill(buf: &mut [u16], x: usize, y: usize, w: usize, h: usize, c: u16) {
    for yy in y..(y + h).min(SH) {
        for xx in x..(x + w).min(SW) {
            buf[yy * SW + xx] = c;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx(k: u8) -> usize {
        CELLS.iter().position(|&c| c == k).unwrap()
    }

    fn keys(s: &[u8], c: &mut Calc) {
        for &k in s {
            c.press(idx(k));
        }
    }

    #[test]
    fn chain_is_left_to_right() {
        let mut c = Calc::new();
        keys(b"2+3*4=", &mut c);
        assert_eq!(c.value(), 20);
        assert!(!c.is_err());
    }

    #[test]
    fn subtract_negative_result() {
        let mut c = Calc::new();
        keys(b"9-12=", &mut c);
        assert_eq!(c.value(), -3);
    }

    #[test]
    fn divide_by_zero_is_error_until_clear() {
        let mut c = Calc::new();
        keys(b"7/0=", &mut c);
        assert!(c.is_err());
        keys(b"1", &mut c); // ignorato in errore
        assert_eq!(c.value(), 0);
        keys(b"C5=", &mut c);
        assert!(!c.is_err());
        assert_eq!(c.value(), 5);
    }

    #[test]
    fn digits_are_capped() {
        let mut c = Calc::new();
        keys(b"123456789012", &mut c);
        assert!(c.value() < 1_000_000_000);
    }

    #[test]
    fn overflow_is_error() {
        let mut c = Calc::new();
        keys(b"99999999*99999999=", &mut c);
        assert!(c.is_err());
    }

    #[test]
    fn keypad_moves_cursor_and_presses() {
        let mut c = Calc::new();
        assert!(c.step(KEY_RIGHT)); // cursore su 8
        assert_eq!(c.cursor(), 1);
        assert!(!c.step(KEY_RIGHT)); // tenuto premuto: nessun passo
        c.step(0);
        assert!(c.step(KEY_A)); // preme 8
        assert_eq!(c.value(), 8);
        c.step(0);
        c.step(KEY_LEFT);
        c.step(0);
        c.step(KEY_LEFT); // colonna 0 -> 3 (wrap a sinistra)
        assert_eq!(c.cursor() % 4, 3);
    }

    #[test]
    fn draw_uses_only_the_four_tints() {
        let mut c = Calc::new();
        keys(b"12-3=", &mut c);
        let mut buf = [0u16; SW * SH];
        c.draw(&mut buf);
        for px in buf.iter() {
            assert!([T0, T1, T2].contains(px), "fuori tavolozza: {px:#06x}");
        }
    }
}
