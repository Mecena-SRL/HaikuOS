//! Creature, tipi, mosse e battaglia a turni. `no_std`, nessuna allocazione.

use crate::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Normal,
    Fire,
    Water,
    Grass,
}

pub struct Species {
    pub name: &'static str,
    pub kind: Kind,
    pub hp: i32,
    pub atk: i32,
    pub def: i32,
    pub spd: i32,
    /// Indici in `MOVES`.
    pub moves: [usize; 2],
}

pub struct Move {
    pub name: &'static str,
    pub kind: Kind,
    pub power: i32,
}

pub static SPECIES: [Species; 4] = [
    Species { name: "FLAMEL", kind: Kind::Fire, hp: 22, atk: 9, def: 6, spd: 8, moves: [0, 3] },
    Species { name: "AQUO", kind: Kind::Water, hp: 24, atk: 8, def: 7, spd: 6, moves: [1, 3] },
    Species { name: "LEAFO", kind: Kind::Grass, hp: 23, atk: 8, def: 7, spd: 7, moves: [2, 3] },
    Species { name: "RATTO", kind: Kind::Normal, hp: 14, atk: 6, def: 4, spd: 9, moves: [3, 3] },
];

pub static MOVES: [Move; 4] = [
    Move { name: "EMBER", kind: Kind::Fire, power: 10 },
    Move { name: "BOLLA", kind: Kind::Water, power: 10 },
    Move { name: "FRUSTA", kind: Kind::Grass, power: 10 },
    Move { name: "PLACCA", kind: Kind::Normal, power: 8 },
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Creature {
    pub sp: usize,
    pub hp: i32,
}

impl Creature {
    pub fn new(sp: usize) -> Self {
        Self { sp, hp: SPECIES[sp].hp }
    }
    pub fn s(&self) -> &'static Species {
        &SPECIES[self.sp]
    }
}

/// Testo a lunghezza fissa per messaggi di battaglia (niente `String`).
#[derive(Clone, Copy)]
pub struct Msg {
    buf: [u8; 128],
    len: usize,
}

impl Default for Msg {
    fn default() -> Self {
        Self::new()
    }
}

impl Msg {
    pub fn new() -> Self {
        Self { buf: [0; 128], len: 0 }
    }
    pub fn clear(&mut self) {
        self.len = 0;
    }
    fn byte(&mut self, b: u8) {
        if self.len < self.buf.len() {
            self.buf[self.len] = b;
            self.len += 1;
        }
    }
    pub fn push(&mut self, s: &str) {
        s.bytes().for_each(|b| self.byte(b));
    }
    pub fn num(&mut self, n: i32) {
        let mut d = [0u8; 10];
        let (mut i, mut v) = (0, n.max(0) as u32);
        loop {
            d[i] = b'0' + (v % 10) as u8;
            v /= 10;
            i += 1;
            if v == 0 {
                break;
            }
        }
        while i > 0 {
            i -= 1;
            self.byte(d[i]);
        }
    }
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Menu,
    Moves,
    Done,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Win,
    Caught,
    Ran,
    Lose,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Act {
    /// Indice della mossa (0 o 1) della creatura attiva.
    Move(usize),
    Catch,
    Run,
}

#[derive(Clone, Copy)]
pub struct Battle {
    pub foe: Creature,
    pub trainer: bool,
    pub phase: Phase,
    pub cursor: usize,
    pub msg: Msg,
    pub outcome: Option<Outcome>,
}

fn eff(m: Kind, d: Kind) -> i32 {
    use Kind::*;
    match (m, d) {
        (Fire, Grass) | (Water, Fire) | (Grass, Water) => 200,
        (Fire, Water) | (Water, Grass) | (Grass, Fire) => 50,
        _ => 100,
    }
}

pub fn damage(a: &Creature, mv: usize, d: &Creature) -> i32 {
    let m = &MOVES[mv];
    let base = a.s().atk * m.power / (d.s().def + 4);
    (base * eff(m.kind, d.s().kind) / 100).max(1)
}

impl Battle {
    pub fn new(foe: Creature, trainer: bool) -> Self {
        let mut msg = Msg::new();
        msg.push(if trainer { "IL TRAINER MANDA " } else { "UN SELVATICO " });
        msg.push(foe.s().name);
        msg.push("!");
        Self { foe, trainer, phase: Phase::Menu, cursor: 0, msg, outcome: None }
    }

    pub fn say(&mut self, s: &str) {
        self.msg.clear();
        self.msg.push(s);
    }

    fn finish(&mut self, o: Outcome, text: &str) {
        self.outcome = Some(o);
        self.phase = Phase::Done;
        self.msg.push(text);
    }

    /// Un turno: l'azione del giocatore (su `me`) e la risposta del nemico.
    pub fn turn(&mut self, me: &mut Creature, a: Act, rng: &mut Rng) {
        self.msg.clear();
        match a {
            Act::Run if self.trainer => return self.say("NON PUOI FUGGIRE DA UN TRAINER!"),
            Act::Run => return self.finish(Outcome::Ran, "SEI FUGGITO!"),
            Act::Catch if self.trainer => return self.say("NON RUBI LE CREATURE ALTRUI!"),
            Act::Catch => {
                let max = self.foe.s().hp;
                let rate = 30 + (max - self.foe.hp) * 60 / max;
                if rng.below(100) < rate as u32 {
                    return self.finish(Outcome::Caught, "CATTURATO!");
                }
                self.msg.push("NON RIUSCITA!\n");
            }
            Act::Move(_) => {}
        }
        let mine = match a {
            Act::Move(i) => Some(me.s().moves[i]),
            _ => None,
        };
        self.exchange(me, mine, rng);
    }

    fn exchange(&mut self, me: &mut Creature, mine: Option<usize>, rng: &mut Rng) {
        let foe_move = self.foe.s().moves[rng.below(2) as usize];
        let (mut p, mut f) = (*me, self.foe);
        let p_first = p.s().spd >= f.s().spd;
        for step in 0..2 {
            let by_me = (step == 0) == p_first;
            let Some(mv) = (if by_me { mine } else { Some(foe_move) }) else { continue };
            let (att, def) = if by_me { (&p, &mut f) } else { (&f, &mut p) };
            if att.hp <= 0 || def.hp <= 0 {
                continue;
            }
            self.msg.push(att.s().name);
            self.msg.push(" USA ");
            self.msg.push(MOVES[mv].name);
            self.msg.push("!\n");
            if rng.below(10) == 0 {
                self.msg.push("MA SBAGLIA!\n");
                continue;
            }
            let dmg = damage(att, mv, def);
            def.hp -= dmg;
            self.msg.push(def.s().name);
            self.msg.push(" -");
            self.msg.num(dmg);
            self.msg.push(" PS\n");
            if def.hp <= 0 {
                self.msg.push(def.s().name);
                self.msg.push(" KO!\n");
            }
        }
        *me = p;
        self.foe = f;
        if self.foe.hp <= 0 {
            self.finish(Outcome::Win, "\nVITTORIA!");
        } else {
            self.phase = Phase::Menu;
            self.cursor = 0;
        }
    }
}
