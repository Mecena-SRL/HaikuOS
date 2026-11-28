//! Giochi di carte: mazzo francese (`fr`), napoletano (`na`) e stile Gwent (`gwent`).
//! `no_std`, senza allocazione. Disegno vettoriale in `draw`, input = azioni (`act`).

#![cfg_attr(not(test), no_std)]

pub mod draw;
mod font;
pub mod fr;
pub mod gwent;
pub mod na;

pub use draw::{Pic, Ui};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Card {
    pub r: u8,
    pub s: u8,
}

/// Pila a capacità fissa. `c[..n]` sono le carte, `c[n-1]` è la cima.
#[derive(Clone, Copy)]
pub struct Pile<const N: usize> {
    pub c: [Card; N],
    pub n: usize,
}

impl<const N: usize> Pile<N> {
    pub const EMPTY: Self = Self { c: [Card { r: 0, s: 0 }; N], n: 0 };

    pub fn push(&mut self, x: Card) {
        if self.n < N {
            self.c[self.n] = x;
            self.n += 1;
        }
    }
    pub fn pop(&mut self) -> Option<Card> {
        if self.n == 0 {
            None
        } else {
            self.n -= 1;
            Some(self.c[self.n])
        }
    }
    pub fn last(&self) -> Option<Card> {
        if self.n == 0 { None } else { Some(self.c[self.n - 1]) }
    }
    pub fn remove(&mut self, i: usize) -> Card {
        let c = self.c[i];
        self.c.copy_within(i + 1..self.n, i);
        self.n -= 1;
        c
    }
    /// Toglie la carta in fondo (indice 0).
    pub fn front(&mut self) -> Option<Card> {
        if self.n == 0 { None } else { Some(self.remove(0)) }
    }
    pub fn s(&self) -> &[Card] {
        &self.c[..self.n]
    }
    pub fn clear(&mut self) {
        self.n = 0;
    }
}

/// Generatore xorshift32, deterministico dato il seme.
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng(seed | 1)
    }
    pub fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() as usize) % n
    }
}

/// Mazzo francese (52 carte, semi 0..3 = picche, cuori, quadri, fiori)
/// oppure napoletano (40 carte, semi 0..3 = coppe, denari, spade, bastoni).
pub fn deck(napoli: bool) -> Pile<52> {
    let mut d = Pile::EMPTY;
    let max = if napoli { 10 } else { 13 };
    for s in 0..4 {
        for r in 1..=max {
            d.push(Card { r, s });
        }
    }
    d
}

pub fn shuffle<const N: usize>(p: &mut Pile<N>, rng: &mut Rng) {
    for i in (1..p.n).rev() {
        let j = rng.below(i + 1);
        p.c.swap(i, j);
    }
}

/// Azione: kind (16 bit alti), arg1, arg2. `NEW` = nuova partita.
pub const NEW: u32 = 0xFF_0000;
pub const fn act(k: u32, a: u32, b: u32) -> u32 {
    k << 16 | (a & 0xFF) << 8 | (b & 0xFF)
}
pub const fn kind(a: u32) -> u32 {
    a >> 16
}
pub const fn arg1(a: u32) -> u32 {
    (a >> 8) & 0xFF
}
pub const fn arg2(a: u32) -> u32 {
    a & 0xFF
}

pub trait Game {
    /// Applica un'azione (tasto o clic). `NEW` riavvia la partita.
    fn act(&mut self, a: u32);
    /// Disegna il frame e registra i punti cliccabili in `ui`.
    fn draw(&self, p: &mut Pic, ui: &mut Ui);
}
