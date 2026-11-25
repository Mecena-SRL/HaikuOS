//! Interfaccia della calcolatrice: griglia di tasti con la croce, A preme, superficie propria.
//! La logica è in `calc.rs`. B chiude la finestra (gestito dal desktop).
#![allow(dead_code)]

use crate::calc::{format, Calc, Op};
use crate::desktop::{T0, T1, T2};
use crate::font;

pub const SW: usize = 104;
pub const SH: usize = 74;
const COLS: usize = 4;
const ROWS: usize = 5;
const CELL_W: usize = 26;
const CELL_H: usize = 12;
const GRID_Y: usize = 14;

pub const KEY_A: u16 = 1 << 0;
pub const KEY_RIGHT: u16 = 1 << 4;
pub const KEY_LEFT: u16 = 1 << 5;
pub const KEY_UP: u16 = 1 << 6;
pub const KEY_DOWN: u16 = 1 << 7;

/// Etichette della griglia 4x5. Spazio = cella vuota, non premibile.
pub const CELLS: [u8; COLS * ROWS] = [
    b'7', b'8', b'9', b'/', //
    b'4', b'5', b'6', b'*', //
    b'1', b'2', b'3', b'-', //
    b'0', b'.', b'=', b'+', //
    b'C', b' ', b' ', b' ',
];

pub struct CalcUi {
    cursor: usize,
    calc: Calc,
    prev: u16,
}

impl CalcUi {
    pub const fn new() -> Self {
        Self { cursor: 0, calc: Calc::new(), prev: 0 }
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn calc(&self) -> &Calc {
        &self.calc
    }

    /// Preme la cella `i`.
    pub fn press(&mut self, i: usize) {
        match CELLS[i % CELLS.len()] {
            d @ b'0'..=b'9' => self.calc.digit(d - b'0'),
            b'.' => self.calc.point(),
            b'+' => self.calc.op(Op::Add),
            b'-' => self.calc.op(Op::Sub),
            b'*' => self.calc.op(Op::Mul),
            b'/' => self.calc.op(Op::Div),
            b'=' => self.calc.equals(),
            b'C' => self.calc.clear(),
            _ => {}
        }
    }

    /// Un passo di input. Ritorna true se bisogna ridisegnare.
    pub fn step(&mut self, keys: u16) -> bool {
        let pressed = keys & !self.prev;
        self.prev = keys;
        let row = self.cursor / COLS;
        let col = self.cursor % COLS;
        if pressed & KEY_RIGHT != 0 {
            self.cursor = row * COLS + (col + 1) % COLS;
        }
        if pressed & KEY_LEFT != 0 {
            self.cursor = row * COLS + (col + COLS - 1) % COLS;
        }
        if pressed & KEY_DOWN != 0 {
            self.cursor = (self.cursor + COLS) % (COLS * ROWS);
        }
        if pressed & KEY_UP != 0 {
            self.cursor = (self.cursor + COLS * ROWS - COLS) % (COLS * ROWS);
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
        // Display in alto a destra: numero o ERR.
        let mut text = [0u8; 12];
        let n = if self.calc.err.is_some() {
            text[..3].copy_from_slice(b"ERR");
            3
        } else {
            format(self.calc.shown(), &mut text)
        };
        let x = (SW - 3).saturating_sub(font::text_width(&text[..n]));
        font::draw(buf, SW, SH, x, 4, &text[..n], T1);

        for i in 0..COLS * ROWS {
            if CELLS[i] == b' ' {
                continue;
            }
            let cx = 2 + (i % COLS) * CELL_W;
            let cy = GRID_Y + (i / COLS) * CELL_H;
            let (bg, fg) = if i == self.cursor { (T1, T0) } else { (T2, T0) };
            fill(buf, cx, cy, CELL_W - 2, CELL_H - 1, bg);
            let lbl = [CELLS[i]];
            let lx = cx + (CELL_W - 2 - font::GLYPH_W) / 2;
            let ly = cy + (CELL_H - 1 - font::GLYPH_H) / 2;
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

    fn tap(ui: &mut CalcUi, s: &[u8]) {
        for &k in s {
            ui.press(idx(k));
        }
    }

    #[test]
    fn keypad_presses_drive_the_calculator() {
        let mut ui = CalcUi::new();
        tap(&mut ui, b"2+3*4=");
        assert_eq!(ui.calc().shown(), 2000); // 20,00 in centesimi
        tap(&mut ui, b"C");
        assert_eq!(ui.calc().shown(), 0);
    }

    #[test]
    fn cursor_moves_and_wraps_inside_rows() {
        let mut ui = CalcUi::new();
        assert!(ui.step(KEY_RIGHT));
        assert_eq!(ui.cursor(), 1);
        assert!(!ui.step(KEY_RIGHT)); // tenuto premuto: nessun passo
        ui.step(0);
        ui.step(KEY_LEFT);
        ui.step(0);
        ui.step(KEY_LEFT); // colonna 0 -> 3 della stessa riga
        assert_eq!(ui.cursor(), 3);
    }

    #[test]
    fn a_presses_the_selected_key() {
        let mut ui = CalcUi::new();
        assert!(ui.step(KEY_A)); // cella 0 = 7
        assert_eq!(ui.calc().shown(), 700);
    }

    #[test]
    fn draw_uses_only_the_three_tints_on_the_surface() {
        let mut ui = CalcUi::new();
        tap(&mut ui, b"12-3=");
        let mut buf = [0u16; SW * SH];
        ui.draw(&mut buf);
        for px in buf.iter() {
            assert!([T0, T1, T2].contains(px), "fuori tavolozza: {px:#06x}");
        }
    }

    #[test]
    fn grid_fits_inside_surface() {
        assert!(GRID_Y + ROWS * CELL_H <= SH);
        assert!(COLS * CELL_W <= SW);
    }
}
