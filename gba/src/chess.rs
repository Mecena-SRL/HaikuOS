//! Scacchi per il GBA: stessa logica di sim/src/chess.rs, senza allocazione (array fissi).
//! Mosse legali senza arrocco ed en passant. Promozione automatica a donna.
//! Un pezzo è un byte: tipo (1..=6) | colore << 3. 0 = casa vuota.

pub const WHITE: u8 = 0;
pub const BLACK: u8 = 1;
pub const PAWN: u8 = 1;
pub const KNIGHT: u8 = 2;
pub const BISHOP: u8 = 3;
pub const ROOK: u8 = 4;
pub const QUEEN: u8 = 5;
pub const KING: u8 = 6;

pub const MAX_MOVES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    pub from: u8,
    pub to: u8,
}

pub struct Moves {
    pub m: [Move; MAX_MOVES],
    pub n: usize,
}

impl Moves {
    pub const fn new() -> Self {
        Moves { m: [Move { from: 0, to: 0 }; MAX_MOVES], n: 0 }
    }
    fn push(&mut self, from: u8, to: u8) {
        if self.n < MAX_MOVES {
            self.m[self.n] = Move { from, to };
            self.n += 1;
        }
    }
    pub fn contains(&self, mv: Move) -> bool {
        self.m[..self.n].contains(&mv)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Ongoing,
    Check,
    Checkmate(u8),
    Stalemate,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Board {
    pub sq: [u8; 64],
    pub turn: u8,
}

const KNIGHT_D: [(i8, i8); 8] = [(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)];
const KING_D: [(i8, i8); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];
const DIAG_D: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
const ORTH_D: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

pub fn piece(kind: u8, color: u8) -> u8 {
    kind | (color << 3)
}

fn kind(p: u8) -> u8 {
    p & 7
}

fn color(p: u8) -> u8 {
    p >> 3
}

/// Casa vicina in direzione (df, dr), se resta sulla scacchiera.
fn step(s: u8, df: i8, dr: i8) -> Option<u8> {
    let f = (s % 8) as i8 + df;
    let r = (s / 8) as i8 + dr;
    if (0..8).contains(&f) && (0..8).contains(&r) {
        Some((r * 8 + f) as u8)
    } else {
        None
    }
}

impl Board {
    pub const fn empty() -> Self {
        Board { sq: [0; 64], turn: WHITE }
    }

    pub fn start() -> Self {
        let mut b = Board::empty();
        let back = [ROOK, KNIGHT, BISHOP, QUEEN, KING, BISHOP, KNIGHT, ROOK];
        for f in 0..8u8 {
            b.sq[f as usize] = piece(back[f as usize], WHITE);
            b.sq[(8 + f) as usize] = piece(PAWN, WHITE);
            b.sq[(48 + f) as usize] = piece(PAWN, BLACK);
            b.sq[(56 + f) as usize] = piece(back[f as usize], BLACK);
        }
        b
    }

    pub fn at(&self, s: u8) -> u8 {
        self.sq[s as usize]
    }

    /// Mosse legali di chi deve muovere, in `out`.
    pub fn legal(&self, out: &mut Moves) {
        out.n = 0;
        let mut ps = Moves::new();
        self.pseudo(self.turn, &mut ps);
        for i in 0..ps.n {
            let mv = ps.m[i];
            let mut b = *self;
            b.apply(mv);
            if !b.in_check(self.turn) {
                out.push(mv.from, mv.to);
            }
        }
    }

    /// Esegue la mossa solo se è legale. `false` se non lo è (la scacchiera non cambia).
    pub fn play(&mut self, mv: Move) -> bool {
        let mut ms = Moves::new();
        self.legal(&mut ms);
        if ms.contains(mv) {
            self.apply(mv);
            true
        } else {
            false
        }
    }

    pub fn outcome(&self) -> Outcome {
        let mut ms = Moves::new();
        self.legal(&mut ms);
        let check = self.in_check(self.turn);
        if ms.n > 0 {
            return if check { Outcome::Check } else { Outcome::Ongoing };
        }
        if check {
            Outcome::Checkmate(self.turn ^ 1)
        } else {
            Outcome::Stalemate
        }
    }

    pub fn in_check(&self, c: u8) -> bool {
        let king = piece(KING, c);
        let k = match (0..64u8).find(|&s| self.at(s) == king) {
            Some(k) => k,
            None => return false,
        };
        let mut ps = Moves::new();
        self.pseudo(c ^ 1, &mut ps);
        ps.m[..ps.n].iter().any(|m| m.to == k)
    }

    /// Esecuzione senza controlli: usare solo con mosse già verificate.
    fn apply(&mut self, mv: Move) {
        let mut p = self.sq[mv.from as usize];
        self.sq[mv.from as usize] = 0;
        if kind(p) == PAWN && (mv.to / 8 == 0 || mv.to / 8 == 7) {
            p = piece(QUEEN, color(p));
        }
        self.sq[mv.to as usize] = p;
        self.turn ^= 1;
    }

    /// Mosse pseudo-legali (senza controllo del re) di un colore.
    fn pseudo(&self, c: u8, out: &mut Moves) {
        for from in 0..64u8 {
            let p = self.at(from);
            if p == 0 || color(p) != c {
                continue;
            }
            let own = |s: u8| self.at(s) != 0 && color(self.at(s)) == c;
            match kind(p) {
                PAWN => {
                    let (dir, start): (i8, u8) = if c == WHITE { (1, 1) } else { (-1, 6) };
                    if let Some(one) = step(from, 0, dir) {
                        if self.at(one) == 0 {
                            out.push(from, one);
                            if from / 8 == start {
                                if let Some(two) = step(one, 0, dir) {
                                    if self.at(two) == 0 {
                                        out.push(from, two);
                                    }
                                }
                            }
                        }
                    }
                    for df in [-1i8, 1] {
                        if let Some(t) = step(from, df, dir) {
                            let q = self.at(t);
                            if q != 0 && color(q) != c {
                                out.push(from, t);
                            }
                        }
                    }
                }
                KNIGHT => {
                    for (df, dr) in KNIGHT_D {
                        if let Some(t) = step(from, df, dr) {
                            if !own(t) {
                                out.push(from, t);
                            }
                        }
                    }
                }
                KING => {
                    for (df, dr) in KING_D {
                        if let Some(t) = step(from, df, dr) {
                            if !own(t) {
                                out.push(from, t);
                            }
                        }
                    }
                }
                BISHOP => self.slide(from, c, &DIAG_D, out),
                ROOK => self.slide(from, c, &ORTH_D, out),
                _ => {
                    self.slide(from, c, &DIAG_D, out);
                    self.slide(from, c, &ORTH_D, out);
                }
            }
        }
    }

    fn slide(&self, from: u8, c: u8, dirs: &[(i8, i8)], out: &mut Moves) {
        for &(df, dr) in dirs {
            let mut cur = from;
            while let Some(n) = step(cur, df, dr) {
                let q = self.at(n);
                if q != 0 && color(q) == c {
                    break;
                }
                out.push(from, n);
                if q != 0 {
                    break;
                }
                cur = n;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perft(b: &Board, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        let mut ms = Moves::new();
        b.legal(&mut ms);
        let mut total = 0;
        for i in 0..ms.n {
            let mut n = *b;
            assert!(n.play(ms.m[i]));
            total += perft(&n, depth - 1);
        }
        total
    }

    fn mv(a: u8, b: u8) -> Move {
        Move { from: a, to: b }
    }

    #[test]
    fn perft_posizione_iniziale() {
        let b = Board::start();
        assert_eq!(perft(&b, 2), 400);
        assert_eq!(perft(&b, 3), 8902);
    }

    #[test]
    fn matto_del_pastore() {
        let mut b = Board::start();
        // f2f3 e7e5 g2g4 d8h4
        assert!(b.play(mv(13, 21)));
        assert!(b.play(mv(52, 36)));
        assert!(b.play(mv(14, 30)));
        assert!(b.play(mv(59, 31)));
        assert_eq!(b.outcome(), Outcome::Checkmate(BLACK));
    }

    #[test]
    fn mossa_illegale_rifiutata() {
        let mut b = Board::start();
        let prima = b;
        assert!(!b.play(mv(12, 36))); // e2e5
        assert_eq!(b, prima);
    }

    #[test]
    fn stallo() {
        let mut b = Board::empty();
        b.sq[56] = piece(KING, BLACK); // a8
        b.sq[50] = piece(QUEEN, WHITE); // c7
        b.sq[41] = piece(KING, WHITE); // b6
        b.turn = BLACK;
        assert_eq!(b.outcome(), Outcome::Stalemate);
    }

    #[test]
    fn promozione_a_donna() {
        let mut b = Board::empty();
        b.sq[49] = piece(PAWN, WHITE); // b7
        b.sq[4] = piece(KING, WHITE); // e1
        b.sq[63] = piece(KING, BLACK); // h8
        assert!(b.play(mv(49, 57)));
        assert_eq!(b.at(57), piece(QUEEN, WHITE));
    }
}
