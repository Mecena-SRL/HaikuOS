//! App media sulla superficie 64×48: scacchi (tastierino, 8×8 quadrati da 6 px) e lettore di testo.
//! Solo logica di disegno e di navigazione: niente accesso all'hardware.
//! Regole: docs/apps-media.md.

use crate::chess::{self, Board, Move, BLACK, KING, WHITE};
use crate::glyph::{self, draw_text, LINE, ORIGIN};

pub const W: usize = 64;
pub const H: usize = 48;

const fn rgb(r: u16, g: u16, b: u16) -> u16 {
    r | (g << 5) | (b << 10)
}

pub const BG: u16 = rgb(2, 2, 2);
pub const LIGHT: u16 = rgb(24, 20, 8);
pub const DARK: u16 = rgb(4, 12, 6);
pub const SEL: u16 = rgb(0, 20, 31);
pub const CURSOR: u16 = rgb(31, 0, 0);
pub const WHITE_PC: u16 = rgb(31, 31, 31);
pub const BLACK_PC: u16 = rgb(0, 0, 0);
pub const TEXT: u16 = rgb(31, 31, 31);
pub const PANEL: u16 = rgb(31, 24, 0);

const SQ: usize = 6;
const LETTERS: &[u8; 6] = b"PNBRQK";

/// Tasti del keypad usati dalle app media.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
}

fn fill(px: &mut [u16], pw: usize, ph: usize, x: usize, y: usize, w: usize, h: usize, c: u16) {
    for yy in y..(y + h).min(ph) {
        for xx in x..(x + w).min(pw) {
            px[yy * pw + xx] = c;
        }
    }
}

/// Stato della partita sulla superficie: cursore, pezzo selezionato, scacchiera.
pub struct ChessView {
    pub board: Board,
    pub cursor: u8,
    pub from: Option<u8>,
}

impl ChessView {
    pub fn new() -> Self {
        ChessView { board: Board::start(), cursor: 12, from: None }
    }

    fn own_piece_at_cursor(&self) -> bool {
        let p = self.board.at(self.cursor);
        p != 0 && (p >> 3) == self.board.turn
    }

    fn select_here(&mut self) {
        self.from = if self.own_piece_at_cursor() { Some(self.cursor) } else { None };
    }

    /// Tasto premuto: il cursore si sposta sulla scacchiera, A seleziona o muove, B annulla.
    pub fn key(&mut self, k: Key) {
        let (f, r) = (self.cursor % 8, self.cursor / 8);
        match k {
            Key::Left if f > 0 => self.cursor -= 1,
            Key::Right if f < 7 => self.cursor += 1,
            Key::Up if r < 7 => self.cursor += 8,
            Key::Down if r > 0 => self.cursor -= 8,
            Key::A => match self.from {
                Some(from) => {
                    if self.board.play(Move { from, to: self.cursor }) {
                        self.from = None;
                    } else {
                        self.select_here();
                    }
                }
                None => self.select_here(),
            },
            Key::B => self.from = None,
            _ => {}
        }
    }

    /// Disegna la scacchiera a sinistra e il turno a destra.
    /// Disegna nella superficie `px` larga `pw` e alta `ph`.
    pub fn draw(&self, px: &mut [u16], pw: usize, ph: usize) {
        fill(px, pw, ph, 0, 0, pw, ph, BG);
        for s in 0..64u8 {
            let x = (s % 8) as usize * SQ;
            let y = (7 - s / 8) as usize * SQ;
            let light = (s % 8 + s / 8) % 2 == 1;
            fill(px, pw, ph, x, y, SQ, SQ, if light { LIGHT } else { DARK });
            if self.from == Some(s) {
                fill(px, pw, ph, x, y, SQ, SQ, SEL);
            }
            let p = self.board.at(s);
            if p != 0 {
                let ch = LETTERS[(p & 7) as usize - 1];
                let col = if p >> 3 == WHITE { WHITE_PC } else { BLACK_PC };
                draw_text(px, pw, ph, x + 1, y, &[ch], col);
            }
            if s == self.cursor {
                // Sottolineatura in basso: non tocca le righe della lettera.
                fill(px, pw, ph, x, y + SQ - 1, SQ, 1, CURSOR);
            }
        }
        let turn = if self.board.turn == WHITE { b'W' } else { b'B' };
        draw_text(px, pw, ph, 50, 2, &[turn], TEXT);
        if self.board.in_check(self.board.turn) {
            draw_text(px, pw, ph, 50, 8, b"C", PANEL);
        }
    }

    /// `true` se il re del turno corrente è sotto scacco (per i test e il pannello).
    pub fn king_square(&self, c: u8) -> Option<u8> {
        (0..64u8).find(|&s| self.board.at(s) == chess::piece(KING, c))
    }

    pub fn black_turn(&self) -> bool {
        self.board.turn == BLACK
    }
}

// ---- Lettore di testo ----

pub const COLS: usize = glyph::COLS;
pub const ROWS: usize = glyph::ROWS;
pub const MAX_TEXT: usize = 128;
const MAX_LINES: usize = 130;

/// Righe del testo: la riga `i` è `text[start[i]..end[i]]`.
pub struct Layout {
    pub start: [u8; MAX_LINES],
    pub end: [u8; MAX_LINES],
    pub n: usize,
}

/// A capo per parola, `COLS` caratteri per riga; `\n` forza una riga nuova.
/// Una parola più lunga di `COLS` viene spezzata. Testo oltre `MAX_LINES` righe si tronca.
pub fn layout(text: &[u8]) -> Layout {
    let mut l = Layout { start: [0; MAX_LINES], end: [0; MAX_LINES], n: 0 };
    let len = text.len();
    let mut i = 0usize;
    while l.n < MAX_LINES {
        let mut end = i;
        let mut sp = None;
        while end < len && end - i < COLS && text[end] != b'\n' {
            if text[end] == b' ' {
                sp = Some(end);
            }
            end += 1;
        }
        let (line_end, next) = if end == len || text[end] == b'\n' {
            (end, if end < len { end + 1 } else { end })
        } else if text[end] == b' ' {
            (end, end + 1)
        } else if let Some(s) = sp {
            (s, s + 1)
        } else {
            (end, end)
        };
        l.start[l.n] = i as u8;
        l.end[l.n] = line_end as u8;
        l.n += 1;
        if next >= len {
            break;
        }
        i = next;
    }
    l
}

pub struct Reader {
    text: [u8; MAX_TEXT],
    len: usize,
    lay: Layout,
    page: usize,
}

impl Reader {
    pub fn new(src: &[u8]) -> Self {
        let mut text = [0u8; MAX_TEXT];
        let len = src.len().min(MAX_TEXT);
        text[..len].copy_from_slice(&src[..len]);
        let lay = layout(&text[..len]);
        Reader { text, len, lay, page: 0 }
    }

    pub fn pages(&self) -> usize {
        ((self.lay.n + ROWS - 1) / ROWS).max(1)
    }

    pub fn page(&self) -> usize {
        self.page
    }

    /// Tasto Destra: pagina successiva. `false` se è l'ultima.
    pub fn next(&mut self) -> bool {
        if self.page + 1 < self.pages() {
            self.page += 1;
            true
        } else {
            false
        }
    }

    /// Tasto Sinistra: pagina precedente. `false` se è la prima.
    pub fn prev(&mut self) -> bool {
        if self.page > 0 {
            self.page -= 1;
            true
        } else {
            false
        }
    }

    pub fn draw(&self, px: &mut [u16], pw: usize, ph: usize) {
        fill(px, pw, ph, 0, 0, pw, ph, BG);
        for r in 0..ROWS {
            let li = self.page * ROWS + r;
            if li >= self.lay.n {
                break;
            }
            let s = self.lay.start[li] as usize;
            let e = self.lay.end[li] as usize;
            draw_text(px, pw, ph, ORIGIN, ORIGIN + r * LINE, &self.text[s..e], TEXT);
        }
        // Indicatore "N OF M" in fondo, sotto le sei righe.
        let ind = [b'0' + (self.page + 1) as u8, b' ', b'O', b'F', b' ', b'0' + self.pages() as u8];
        draw_text(px, pw, ph, ORIGIN, 40, &ind, PANEL);
    }

    pub fn text(&self) -> &[u8] {
        &self.text[..self.len]
    }
}
