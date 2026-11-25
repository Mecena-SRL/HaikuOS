//! Scacchi nel simulatore: logica pura, testabile sull'host, senza I/O.
//! Mosse legali complete tranne arrocco ed en passant (da fare). La promozione è automatica a donna.
//! Regole e scelte: docs/apps-media.md.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn other(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub color: Color,
    pub kind: Kind,
}

/// Casa 0..64: a1 = 0, h1 = 7, a8 = 56, h8 = 63.
pub type Sq = u8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    pub from: Sq,
    pub to: Sq,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Ongoing,
    Check,
    Checkmate(Color),
    Stalemate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    sq: [Option<Piece>; 64],
    pub turn: Color,
}

const KNIGHT: [(i8, i8); 8] = [(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)];
const KING: [(i8, i8); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];
const DIAG: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
const ORTH: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Casa vicina in direzione (df, dr), se resta sulla scacchiera.
fn step(s: Sq, df: i8, dr: i8) -> Option<Sq> {
    let f = (s % 8) as i8 + df;
    let r = (s / 8) as i8 + dr;
    if (0..8).contains(&f) && (0..8).contains(&r) {
        Some((r * 8 + f) as Sq)
    } else {
        None
    }
}

/// Nome di casa, es. "e4".
pub fn name(s: Sq) -> String {
    format!("{}{}", (b'a' + s % 8) as char, s / 8 + 1)
}

/// Casa da nome, es. "e4". `None` se non valida.
pub fn sq(name: &str) -> Option<Sq> {
    let b = name.as_bytes();
    if b.len() != 2 || !(b'a'..=b'h').contains(&b[0]) || !(b'1'..=b'8').contains(&b[1]) {
        return None;
    }
    Some((b[1] - b'1') * 8 + (b[0] - b'a'))
}

impl Board {
    pub fn empty() -> Self {
        Board { sq: [None; 64], turn: Color::White }
    }

    pub fn start() -> Self {
        let mut b = Board::empty();
        let back = [Kind::Rook, Kind::Knight, Kind::Bishop, Kind::Queen, Kind::King, Kind::Bishop, Kind::Knight, Kind::Rook];
        for f in 0..8 {
            b.put(f, Piece { color: Color::White, kind: back[f as usize] });
            b.put(8 + f, Piece { color: Color::White, kind: Kind::Pawn });
            b.put(48 + f, Piece { color: Color::Black, kind: Kind::Pawn });
            b.put(56 + f, Piece { color: Color::Black, kind: back[f as usize] });
        }
        b
    }

    pub fn at(&self, s: Sq) -> Option<Piece> {
        self.sq[s as usize]
    }

    pub fn put(&mut self, s: Sq, p: Piece) {
        self.sq[s as usize] = Some(p);
    }

    /// Mosse legali di chi deve muovere.
    pub fn legal_moves(&self) -> Vec<Move> {
        self.pseudo_of(self.turn)
            .into_iter()
            .filter(|&m| {
                let mut b = self.clone();
                b.apply(m);
                !b.in_check(self.turn)
            })
            .collect()
    }

    /// Esegue la mossa solo se è legale. `false` se non lo è (la scacchiera non cambia).
    pub fn play(&mut self, m: Move) -> bool {
        if self.legal_moves().contains(&m) {
            self.apply(m);
            true
        } else {
            false
        }
    }

    pub fn outcome(&self) -> Outcome {
        let check = self.in_check(self.turn);
        if !self.legal_moves().is_empty() {
            return if check { Outcome::Check } else { Outcome::Ongoing };
        }
        if check {
            Outcome::Checkmate(self.turn.other())
        } else {
            Outcome::Stalemate
        }
    }

    /// `true` se il re di `c` è sotto attacco.
    pub fn in_check(&self, c: Color) -> bool {
        let king = (0..64).find(|&s| self.at(s) == Some(Piece { color: c, kind: Kind::King }));
        match king {
            None => false,
            // Un pedone che avanza non cattura mai la casa vuota: le mosse pseudo-legali
            // dell'avversario che arrivano sul re sono proprio gli attacchi.
            Some(k) => self.pseudo_of(c.other()).iter().any(|m| m.to == k),
        }
    }

    /// Esecuzione senza controlli: usare solo con mosse già verificate.
    fn apply(&mut self, m: Move) {
        let mut p = self.sq[m.from as usize].take().expect("mossa senza pezzo");
        if p.kind == Kind::Pawn && (m.to / 8 == 0 || m.to / 8 == 7) {
            p.kind = Kind::Queen;
        }
        self.sq[m.to as usize] = Some(p);
        self.turn = self.turn.other();
    }

    /// Mosse pseudo-legali (senza controllo del re) di un colore.
    fn pseudo_of(&self, c: Color) -> Vec<Move> {
        let mut v = Vec::new();
        for from in 0..64u8 {
            if let Some(p) = self.at(from).filter(|p| p.color == c) {
                for to in self.targets(from, p) {
                    v.push(Move { from, to });
                }
            }
        }
        v
    }

    fn targets(&self, from: Sq, p: Piece) -> Vec<Sq> {
        let mut t = Vec::new();
        let open = |s: Sq| !matches!(self.at(s), Some(q) if q.color == p.color);
        let enemy = |s: Sq| matches!(self.at(s), Some(q) if q.color != p.color);
        match p.kind {
            Kind::Pawn => {
                let (dir, start) = if p.color == Color::White { (1, 1) } else { (-1, 6) };
                if let Some(one) = step(from, 0, dir).filter(|&s| self.at(s).is_none()) {
                    t.push(one);
                    if from / 8 == start {
                        if let Some(two) = step(one, 0, dir).filter(|&s| self.at(s).is_none()) {
                            t.push(two);
                        }
                    }
                }
                for df in [-1, 1] {
                    if let Some(c) = step(from, df, dir).filter(|&s| enemy(s)) {
                        t.push(c);
                    }
                }
            }
            Kind::Knight => t.extend(KNIGHT.iter().filter_map(|&(df, dr)| step(from, df, dr)).filter(|&s| open(s))),
            Kind::King => t.extend(KING.iter().filter_map(|&(df, dr)| step(from, df, dr)).filter(|&s| open(s))),
            Kind::Bishop => self.slide(from, &DIAG, &open, &mut t),
            Kind::Rook => self.slide(from, &ORTH, &open, &mut t),
            Kind::Queen => {
                self.slide(from, &DIAG, &open, &mut t);
                self.slide(from, &ORTH, &open, &mut t);
            }
        }
        t
    }

    fn slide(&self, from: Sq, dirs: &[(i8, i8)], open: &dyn Fn(Sq) -> bool, t: &mut Vec<Sq>) {
        for &(df, dr) in dirs {
            let mut cur = from;
            while let Some(n) = step(cur, df, dr) {
                if !open(n) {
                    break;
                }
                t.push(n);
                if self.at(n).is_some() {
                    break;
                }
                cur = n;
            }
        }
    }
}
