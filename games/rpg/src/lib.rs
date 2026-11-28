//! Gioco di ruolo in stile giallo: mondo a tile, dialoghi, battaglie a turni.
//! `no_std`, nessuna allocazione. Il rendering è in `draw`, indipendente dalla piattaforma.

#![cfg_attr(not(test), no_std)]

pub mod battle;
pub mod draw;
pub mod world;

use battle::{Act, Battle, Creature, Outcome, Phase};
use world::{NpcKind, NPCS};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Walk,
    Talk,
    Fight,
}

/// Generatore xorshift32: deterministico, niente `std`.
#[derive(Clone, Copy)]
pub struct Rng(pub u32);

impl Rng {
    pub fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
}

/// Direzioni su, giù, sinistra, destra.
const DIRS: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

pub struct Game {
    pub map: u8,
    pub x: i32,
    pub y: i32,
    pub dir: usize,
    pub rng: Rng,
    pub mode: Mode,
    pub party: [Creature; 3],
    pub n: usize,
    pub active: usize,
    pub balls: u32,
    pub beaten: bool,
    pub text: &'static str,
    pub page: usize,
    pub pending: Option<usize>,
    pub battle: Battle,
    /// Fotogrammi della transizione a barre (0 = nessuna).
    pub fade: u32,
}

impl Game {
    pub fn new(seed: u32) -> Self {
        let (x, y) = world::START;
        Self {
            map: 0,
            x,
            y,
            dir: 1,
            rng: Rng(seed | 1),
            mode: Mode::Walk,
            party: [Creature::new(0); 3],
            n: 1,
            active: 0,
            balls: 5,
            beaten: false,
            text: "",
            page: 0,
            pending: None,
            battle: Battle::new(Creature::new(3), false),
            fade: 0,
        }
    }

    pub fn press(&mut self, b: Button) {
        match self.mode {
            Mode::Walk => match b {
                Button::A => self.interact(),
                Button::B => {}
                _ => self.step(dir_of(b)),
            },
            Mode::Talk => {
                if matches!(b, Button::A | Button::B) {
                    self.page += 1;
                    if self.page >= self.text.split('|').count() {
                        self.mode = Mode::Walk;
                        if self.pending.take().is_some() {
                            self.start(Creature::new(2), true);
                        }
                    }
                }
            }
            Mode::Fight => self.fight_key(b),
        }
    }

    /// Avanza la transizione di un passo; chiamare una volta per fotogramma.
    pub fn tick(&mut self) {
        self.fade = self.fade.saturating_sub(1);
    }

    fn start(&mut self, foe: Creature, trainer: bool) {
        self.fade = 16;
        self.battle = Battle::new(foe, trainer);
        self.mode = Mode::Fight;
    }

    fn step(&mut self, d: usize) {
        self.dir = d;
        let (nx, ny) = (self.x + DIRS[d].0, self.y + DIRS[d].1);
        if !world::walkable(self.map, nx, ny) || world::npc_at(self.map, nx, ny).is_some() {
            return;
        }
        self.x = nx;
        self.y = ny;
        match world::tile(self.map, nx, ny) {
            '>' | '<' => {
                if let Some((m, x, y)) = world::warp(self.map, self.x, self.y) {
                    (self.map, self.x, self.y) = (m, x, y);
                    self.fade = 16;
                }
            }
            ',' if self.rng.below(6) == 0 => {
                let sp = [3, 2, 1][self.rng.below(3) as usize];
                self.start(Creature::new(sp), false);
            }
            _ => {}
        }
    }

    fn interact(&mut self) {
        // Prima la direzione in cui si guarda, poi le altre: si gira verso chi si trova accanto.
        let order = [self.dir, 0, 1, 2, 3];
        let found = order.iter().find_map(|&d| {
            world::npc_at(self.map, self.x + DIRS[d].0, self.y + DIRS[d].1).map(|i| (d, i))
        });
        let Some((d, i)) = found else { return };
        self.dir = d;
        let npc = NPCS[i];
        self.page = 0;
        self.mode = Mode::Talk;
        self.text = match npc.kind {
            NpcKind::Trainer if self.beaten => "HAI VINTO! BEL COMBATTIMENTO.",
            _ => npc.text,
        };
        match npc.kind {
            NpcKind::Nurse => self.heal(),
            NpcKind::Trainer if !self.beaten => self.pending = Some(i),
            _ => {}
        }
    }

    fn heal(&mut self) {
        for c in self.party[..self.n].iter_mut() {
            c.hp = c.s().hp;
        }
    }

    fn fight_key(&mut self, b: Button) {
        if self.battle.phase == Phase::Done {
            if matches!(b, Button::A | Button::B) {
                self.end_battle();
            }
            return;
        }
        match self.battle.phase {
            Phase::Menu => match b {
                Button::A => match self.battle.cursor {
                    0 => {
                        self.battle.phase = Phase::Moves;
                        self.battle.cursor = 0;
                    }
                    1 => self.act(Act::Catch),
                    _ => self.act(Act::Run),
                },
                _ => self.battle.cursor = moveit(self.battle.cursor, 3, b),
            },
            Phase::Moves => match b {
                Button::A => self.act(Act::Move(self.battle.cursor)),
                Button::B => {
                    self.battle.phase = Phase::Menu;
                    self.battle.cursor = 0;
                }
                _ => self.battle.cursor = moveit(self.battle.cursor, 2, b),
            },
            Phase::Done => {}
        }
    }

    fn act(&mut self, a: Act) {
        if a == Act::Catch && !self.battle.trainer {
            if self.n >= 3 {
                self.battle.say("SQUADRA PIENA!");
                return;
            }
            if self.balls == 0 {
                self.battle.say("NIENTE PALLE!");
                return;
            }
            self.balls -= 1;
        }
        let mut me = self.party[self.active];
        self.battle.turn(&mut me, a, &mut self.rng);
        self.party[self.active] = me;
        if self.battle.outcome == Some(Outcome::Caught) {
            self.party[self.n] = self.battle.foe;
            self.n += 1;
        }
        if self.battle.outcome.is_none() && me.hp <= 0 {
            match (0..self.n).find(|&i| self.party[i].hp > 0) {
                Some(j) => {
                    self.active = j;
                    self.battle.msg.push("VAI, ");
                    self.battle.msg.push(self.party[j].s().name);
                    self.battle.msg.push("!");
                    self.battle.phase = Phase::Menu;
                    self.battle.cursor = 0;
                }
                None => {
                    self.battle.outcome = Some(Outcome::Lose);
                    self.battle.phase = Phase::Done;
                    self.battle.msg.push("SEI SVENUTO!");
                }
            }
        }
    }

    fn end_battle(&mut self) {
        let bt = self.battle;
        self.mode = Mode::Walk;
        match bt.outcome {
            Some(Outcome::Lose) => {
                self.heal();
                self.active = 0;
                (self.map, self.x, self.y) = (0, world::START.0, world::START.1);
                self.text = "SEI SVENUTO. CREATURE GUARITE.";
                self.fade = 16;
                self.page = 0;
                self.mode = Mode::Talk;
            }
            Some(Outcome::Win) if bt.trainer => {
                self.beaten = true;
                self.text = "HAI VINTO LA SFIDA! SEI IL CAMPIONE!";
                self.page = 0;
                self.mode = Mode::Talk;
            }
            _ => {}
        }
    }
}

fn dir_of(b: Button) -> usize {
    match b {
        Button::Up => 0,
        Button::Down => 1,
        Button::Left => 2,
        _ => 3,
    }
}

/// Scorre la selezione di `n` voci con su/giù.
fn moveit(cur: usize, n: usize, b: Button) -> usize {
    match b {
        Button::Up => (cur + n - 1) % n,
        Button::Down => (cur + 1) % n,
        _ => cur,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_are_16_by_12() {
        for m in 0..3 {
            let rows = world::map(m);
            assert_eq!(rows.len(), 12);
            assert!(rows.iter().all(|r| r.len() == 16), "map {m} row width");
        }
    }

    #[test]
    fn start_and_npcs_are_walkable() {
        let (x, y) = world::START;
        assert!(world::walkable(0, x, y));
        for n in NPCS.iter() {
            assert!(world::walkable(n.map, n.x, n.y), "npc at {},{}", n.x, n.y);
        }
    }

    #[test]
    fn trainer_battle_playthrough() {
        let mut g = Game::new(7);
        // Vai alla sinistra dell'allenatore sul percorso.
        g.map = 1;
        (g.x, g.y, g.dir) = (8, 5, 3);
        g.press(Button::A); // parla
        g.press(Button::A); // pagina 2
        g.press(Button::A); // fine dialogo: parte la lotta
        assert_eq!(g.mode, Mode::Fight);
        for _ in 0..40 {
            if g.mode != Mode::Fight {
                break;
            }
            match g.battle.phase {
                Phase::Menu => g.press(Button::A), // apri le mosse
                Phase::Moves => g.press(Button::A), // EMBER
                Phase::Done => g.press(Button::A), // chiudi
            }
        }
        assert_eq!(g.battle.outcome, Some(Outcome::Win));
        assert!(g.beaten, "il trainer deve risultare sconfitto");
        assert_eq!(g.mode, Mode::Talk);
    }

    #[test]
    fn catch_wild_creature() {
        let mut rng = Rng(99);
        let mut me = Creature::new(0);
        let mut b = Battle::new(Creature { sp: 3, hp: 1 }, false);
        for _ in 0..20 {
            if b.outcome.is_some() {
                break;
            }
            b.turn(&mut me, Act::Catch, &mut rng);
        }
        assert!(matches!(b.outcome, Some(Outcome::Caught) | Some(Outcome::Lose)));
    }

    #[test]
    fn trainer_catch_keeps_ball() {
        let mut g = Game::new(7);
        g.map = 1;
        (g.x, g.y, g.dir) = (8, 5, 3);
        for _ in 0..3 {
            g.press(Button::A);
        }
        assert_eq!(g.mode, Mode::Fight);
        g.press(Button::Down);
        g.press(Button::A);
        g.press(Button::Down);
        g.press(Button::A);
        assert_eq!(g.balls, 5);
    }

    #[test]
    fn full_party_catch_is_refused_without_ball() {
        let mut g = Game::new(3);
        g.n = 3;
        g.mode = Mode::Fight;
        g.battle = Battle::new(Creature { sp: 3, hp: 1 }, false);
        g.battle.cursor = 1;
        g.press(Button::A);
        assert_eq!((g.balls, g.n), (5, 3));
        assert_eq!(g.battle.outcome, None);
    }

    #[test]
    fn b_in_menu_does_nothing() {
        let mut g = Game::new(3);
        g.mode = Mode::Fight;
        g.battle = Battle::new(Creature { sp: 3, hp: 14 }, false);
        g.press(Button::B);
        assert_eq!((g.battle.phase, g.battle.cursor, g.mode), (Phase::Menu, 0, Mode::Fight));
    }

    #[test]
    fn warp_round_trip() {
        let mut g = Game::new(3);
        (g.map, g.x, g.y, g.dir) = (0, 14, 6, 3);
        g.press(Button::Right);
        assert_eq!((g.map, g.x, g.y), (1, 1, 6));
        g.press(Button::Left);
        assert_eq!((g.map, g.x, g.y), (0, 14, 6));
    }

    #[test]
    fn map_borders_are_closed() {
        for m in 0..3 {
            let rows = world::map(m);
            for x in 0..16 {
                for y in [0usize, 11] {
                    assert!("#TH<>".contains(rows[y].as_bytes()[x] as char), "map {m} edge {x},{y}");
                }
            }
            for row in rows.iter() {
                let b = row.as_bytes();
                for &c in [b[0], b[15]].iter() {
                    assert!("#TH<>".contains(c as char), "map {m} side in {row}");
                }
            }
        }
    }

    #[test]
    fn battle_messages_fit_the_box() {
        for seed in 1..200u32 {
            let mut rng = Rng(seed);
            let mut me = Creature::new(0);
            let mut b = Battle::new(Creature::new(1), false);
            for _ in 0..30 {
                if b.outcome.is_some() {
                    break;
                }
                b.turn(&mut me, Act::Move(0), &mut rng);
                let lines = b.msg.as_str().matches('\n').count() + 1;
                assert!(lines <= 6, "seed {seed}: {lines} lines");
                assert!(!b.msg.overflow, "seed {seed}: message overflow");
            }
        }
    }

    #[test]
    fn type_advantage_doubles_damage() {
        let fire = Creature::new(0);
        let grass = Creature::new(2);
        assert!(battle::damage(&fire, 0, &grass) > battle::damage(&fire, 1, &grass));
    }
}
