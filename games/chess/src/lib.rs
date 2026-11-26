//! Regole degli scacchi, `no_std`. Usa `Grid` dell'editor come scacchiera.
//! Manca: arrocco e en passant (da aggiungere). Promozione: sempre a donna.

#![cfg_attr(not(test), no_std)]

use haiku_editor::Grid;

pub const EMPTY: u8 = 0;
pub const PAWN: u8 = 1;
pub const KNIGHT: u8 = 2;
pub const BISHOP: u8 = 3;
pub const ROOK: u8 = 4;
pub const QUEEN: u8 = 5;
pub const KING: u8 = 6;
/// Colore: 0 = bianco, `BLACK` = nero. Pezzo = colore | tipo.
pub const WHITE: u8 = 0;
pub const BLACK: u8 = 8;

pub type Board = Grid<8, 8>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Move {
    pub from: (u8, u8),
    pub to: (u8, u8),
}

impl Move {
    /// Legge notazione tipo "e2e4".
    pub fn parse(s: &str) -> Option<Move> {
        let b = s.as_bytes();
        if b.len() != 4 {
            return None;
        }
        let sq = |f: u8, r: u8| -> Option<(u8, u8)> {
            let (x, y) = (f.wrapping_sub(b'a'), r.wrapping_sub(b'1'));
            (x < 8 && y < 8).then_some((x, y))
        };
        Some(Move { from: sq(b[0], b[1])?, to: sq(b[2], b[3])? })
    }
}

/// Lista di mosse a dimensione fissa (nessuna allocazione).
#[derive(Clone, Copy)]
pub struct Moves {
    pub list: [Move; 256],
    pub len: usize,
}

impl Default for Moves {
    fn default() -> Self {
        Self { list: [Move { from: (0, 0), to: (0, 0) }; 256], len: 0 }
    }
}

impl Moves {
    fn push(&mut self, from: (i32, i32), to: (i32, i32)) {
        if self.len < 256 {
            self.list[self.len] = Move { from: (from.0 as u8, from.1 as u8), to: (to.0 as u8, to.1 as u8) };
            self.len += 1;
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = Move> + '_ {
        self.list[..self.len].iter().copied()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Playing,
    Check,
    Checkmate,
    Stalemate,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Game {
    pub board: Board,
    pub turn: u8,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        const BACK: [u8; 8] = [ROOK, KNIGHT, BISHOP, QUEEN, KING, BISHOP, KNIGHT, ROOK];
        let mut board = Board::default();
        for x in 0..8 {
            board.cells[0][x] = BACK[x] | WHITE;
            board.cells[1][x] = PAWN | WHITE;
            board.cells[6][x] = PAWN | BLACK;
            board.cells[7][x] = BACK[x] | BLACK;
        }
        Self { board, turn: WHITE }
    }

    /// Mosse legali per il giocatore di turno.
    pub fn legal_moves(&self) -> Moves {
        let mut out = Moves::default();
        let mut pseudo = Moves::default();
        gen_pseudo(&self.board, self.turn, &mut pseudo);
        for m in pseudo.iter() {
            let nb = apply(&self.board, m);
            if !in_check(&nb, self.turn) {
                out.list[out.len] = m;
                out.len += 1;
            }
        }
        out
    }

    /// Esegue la mossa se è legale. Ritorna `false` altrimenti.
    pub fn play(&mut self, m: Move) -> bool {
        if !self.legal_moves().iter().any(|x| x == m) {
            return false;
        }
        self.apply_unchecked(m);
        true
    }

    fn apply_unchecked(&mut self, m: Move) {
        self.board = apply(&self.board, m);
        self.turn ^= BLACK;
    }

    pub fn status(&self) -> Status {
        let check = in_check(&self.board, self.turn);
        match (self.legal_moves().len, check) {
            (0, true) => Status::Checkmate,
            (0, false) => Status::Stalemate,
            (_, true) => Status::Check,
            _ => Status::Playing,
        }
    }
}

fn cell(b: &Board, x: i32, y: i32) -> Option<u8> {
    if !(0..8).contains(&x) || !(0..8).contains(&y) {
        return None;
    }
    Some(b.cells[y as usize][x as usize])
}

/// Aggiunge la mossa verso (tx,ty). Ritorna `true` se la casella è vuota (lo slider può continuare).
fn step(b: &Board, color: u8, from: (i32, i32), tx: i32, ty: i32, out: &mut Moves) -> bool {
    let Some(t) = cell(b, tx, ty) else { return false };
    if t != EMPTY && (t & BLACK) == color {
        return false;
    }
    out.push(from, (tx, ty));
    t == EMPTY
}

/// Mosse pseudo-legali: non verifica lo scacco.
fn gen_pseudo(b: &Board, color: u8, out: &mut Moves) {
    const KNIGHT_D: [(i32, i32); 8] = [(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)];
    const DIAG: [(i32, i32); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
    const ORTH: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    const ALL: [(i32, i32); 8] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)];

    for y in 0..8i32 {
        for x in 0..8i32 {
            let p = b.cells[y as usize][x as usize];
            if p == EMPTY || (p & BLACK) != color {
                continue;
            }
            let from = (x, y);
            match p & 7 {
                PAWN => {
                    let (dir, start) = if color == WHITE { (1, 1) } else { (-1, 6) };
                    if cell(b, x, y + dir) == Some(EMPTY) {
                        out.push(from, (x, y + dir));
                        if y == start && cell(b, x, y + 2 * dir) == Some(EMPTY) {
                            out.push(from, (x, y + 2 * dir));
                        }
                    }
                    for dx in [-1, 1] {
                        if let Some(t) = cell(b, x + dx, y + dir) {
                            if t != EMPTY && (t & BLACK) != color {
                                out.push(from, (x + dx, y + dir));
                            }
                        }
                    }
                }
                KNIGHT => {
                    for (dx, dy) in KNIGHT_D {
                        step(b, color, from, x + dx, y + dy, out);
                    }
                }
                KING => {
                    for (dx, dy) in ALL {
                        step(b, color, from, x + dx, y + dy, out);
                    }
                }
                kind => {
                    let dirs: &[(i32, i32)] = match kind {
                        BISHOP => &DIAG,
                        ROOK => &ORTH,
                        _ => &ALL, // donna
                    };
                    for &(dx, dy) in dirs {
                        let (mut tx, mut ty) = (x + dx, y + dy);
                        while step(b, color, from, tx, ty, out) {
                            tx += dx;
                            ty += dy;
                        }
                    }
                }
            }
        }
    }
}

/// Applica una mossa senza controlli (promozione automatica a donna).
fn apply(b: &Board, m: Move) -> Board {
    let mut nb = *b;
    let (fx, fy) = (m.from.0 as usize, m.from.1 as usize);
    let (tx, ty) = (m.to.0 as usize, m.to.1 as usize);
    let p = nb.cells[fy][fx];
    nb.cells[fy][fx] = EMPTY;
    nb.cells[ty][tx] = if p & 7 == PAWN && (ty == 0 || ty == 7) { (p & BLACK) | QUEEN } else { p };
    nb
}

/// Il re del colore `color` è sotto attacco? Basta confrontare le mosse pseudo-legali
/// dell'avversario: avanzate di pedone verso il re non possono esistere (la casella è occupata).
fn in_check(b: &Board, color: u8) -> bool {
    let mut king = None;
    for y in 0..8 {
        for x in 0..8 {
            if b.cells[y][x] == KING | color {
                king = Some((x as u8, y as u8));
            }
        }
    }
    let Some(k) = king else { return false };
    let mut attacks = Moves::default();
    gen_pseudo(b, color ^ BLACK, &mut attacks);
    let hit = attacks.iter().any(|m| m.to == k);
    hit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perft(g: &Game, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        let moves = g.legal_moves();
        moves.iter().map(|m| {
            let mut c = *g;
            c.apply_unchecked(m);
            perft(&c, depth - 1)
        }).sum()
    }

    #[test]
    fn perft_start() {
        let g = Game::new();
        assert_eq!(perft(&g, 1), 20);
        assert_eq!(perft(&g, 2), 400);
        assert_eq!(perft(&g, 3), 8902);
    }

    #[test]
    fn fools_mate() {
        let mut g = Game::new();
        for m in ["f2f3", "e7e5", "g2g4", "d8h4"] {
            assert!(g.play(Move::parse(m).unwrap()), "{m}");
        }
        assert_eq!(g.status(), Status::Checkmate);
    }

    #[test]
    fn illegal_move_rejected() {
        let mut g = Game::new();
        assert!(!g.play(Move::parse("e2e5").unwrap()));
        assert_eq!(g.turn, WHITE);
    }

    #[test]
    fn parse_squares() {
        assert_eq!(Move::parse("a1h8"), Some(Move { from: (0, 0), to: (7, 7) }));
        assert_eq!(Move::parse("i1a1"), None);
    }
}
