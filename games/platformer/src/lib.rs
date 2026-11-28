//! Platformer a scorrimento laterale. Solo logica: nessun rendering, nessun I/O.
//! Coordinate: posizioni e velocità in sotto-pixel (`SUB` per pixel), dimensioni in pixel.
#![cfg_attr(not(test), no_std)]

pub mod bot;
pub mod levels;
pub mod render;

pub const SUB: i32 = 16;
pub const TILE: i32 = 16;
pub const VIEW_W: i32 = 256;
pub const VIEW_H: i32 = 240;
pub const MAX_W: usize = 128;
pub const MAX_H: usize = 15;
pub const MAX_MOBS: usize = 32;
pub const LIVES_START: u8 = 3;

pub const BTN_LEFT: u8 = 1;
pub const BTN_RIGHT: u8 = 2;
pub const BTN_JUMP: u8 = 4;
pub const BTN_RUN: u8 = 8;
pub const BTN_START: u8 = 16;

const GRAV: i32 = 8;
const MAX_FALL: i32 = 96;
const JUMP_V: i32 = -128;
const JUMP_CUT: i32 = -40;
const WALK: i32 = 24;
const RUN: i32 = 40;
const ACC: i32 = 4;
const FRIC: i32 = 4;
const MOB_SPEED: i32 = 16;
const STAR_SPEED: i32 = 24;
const STAR_FRAMES: u32 = 600;
const HURT_FRAMES: u32 = 90;
const CLEAR_FRAMES: u32 = 150;
const MOB_SIZE: i32 = 14;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reward {
    Coin,
    Mushroom,
    Star,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    Air,
    Ground,
    Brick,
    Question(Reward),
    Used,
    Coin,
    Flag,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MobKind {
    Fungo,
    Mushroom,
    Star,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Map,
    Playing,
    Clear,
    Over,
    Won,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Body {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct Mob {
    pub kind: MobKind,
    pub b: Body,
    pub dir: i32,
    pub alive: bool,
}

const NO_MOB: Mob = Mob {
    kind: MobKind::Fungo,
    b: Body {
        x: 0,
        y: 0,
        vx: 0,
        vy: 0,
        w: 0,
        h: 0,
    },
    dir: 1,
    alive: false,
};

#[derive(Clone, Copy, Debug)]
pub struct Player {
    pub b: Body,
    pub on_ground: bool,
    pub big: bool,
    pub star: u32,
    pub hurt: u32,
    pub facing: i32,
}

impl Player {
    fn new(x: i32, y: i32) -> Player {
        Player {
            b: Body {
                x: x * SUB,
                y: y * SUB,
                vx: 0,
                vy: 0,
                w: 12,
                h: 14,
            },
            on_ground: false,
            big: false,
            star: 0,
            hurt: 0,
            facing: 1,
        }
    }
}

/// Livello parsato da testo ASCII (vedi `levels.rs`).
pub struct Level {
    pub w: usize,
    pub h: usize,
    pub tiles: [[Tile; MAX_W]; MAX_H],
    pub spawn: (i32, i32),
    pub mobs: [(MobKind, i32, i32); MAX_MOBS],
    pub n_mobs: usize,
}

impl Level {
    pub fn parse(src: &str) -> Option<Level> {
        let mut l = Level {
            w: 0,
            h: 0,
            tiles: [[Tile::Air; MAX_W]; MAX_H],
            spawn: (-1, -1),
            mobs: [(MobKind::Fungo, 0, 0); MAX_MOBS],
            n_mobs: 0,
        };
        for (y, row) in src.lines().filter(|r| !r.is_empty()).enumerate() {
            if y >= MAX_H {
                return None;
            }
            l.h = y + 1;
            for (x, c) in row.chars().enumerate() {
                if x >= MAX_W {
                    return None;
                }
                l.w = l.w.max(x + 1);
                let (px, py) = (x as i32 * TILE, y as i32 * TILE);
                l.tiles[y][x] = match c {
                    '.' => Tile::Air,
                    '#' => Tile::Ground,
                    'B' => Tile::Brick,
                    '?' => Tile::Question(Reward::Coin),
                    'Q' => Tile::Question(Reward::Mushroom),
                    'S' => Tile::Question(Reward::Star),
                    'c' => Tile::Coin,
                    'F' => Tile::Flag,
                    'P' => {
                        l.spawn = (px, py);
                        Tile::Air
                    }
                    'g' => {
                        if l.n_mobs == MAX_MOBS {
                            return None;
                        }
                        l.mobs[l.n_mobs] = (MobKind::Fungo, px, py);
                        l.n_mobs += 1;
                        Tile::Air
                    }
                    _ => return None,
                };
            }
        }
        if l.w == 0 || l.spawn.0 < 0 {
            return None;
        }
        Some(l)
    }

    /// Tile alle coordinate di tile. Fuori dai lati: muro. Sopra: aria. Sotto: vuoto (buca).
    pub fn tile(&self, tx: i32, ty: i32) -> Tile {
        if tx < 0 || tx >= self.w as i32 {
            return Tile::Ground;
        }
        if ty < 0 || ty >= self.h as i32 {
            return Tile::Air;
        }
        self.tiles[ty as usize][tx as usize]
    }

    pub fn solid(&self, tx: i32, ty: i32) -> bool {
        matches!(
            self.tile(tx, ty),
            Tile::Ground | Tile::Brick | Tile::Question(_) | Tile::Used
        )
    }

    /// Rettangolo in sotto-pixel (x, y, w, h) contro i tile solidi.
    fn blocked(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        let (x0, x1, y0, y1) = span_xy(x, y, w, h);
        (y0..=y1).any(|ty| (x0..=x1).any(|tx| self.solid(tx, ty)))
    }
}

fn span_xy(x: i32, y: i32, w: i32, h: i32) -> (i32, i32, i32, i32) {
    let t = TILE * SUB;
    (
        x.div_euclid(t),
        (x + w - 1).div_euclid(t),
        y.div_euclid(t),
        (y + h - 1).div_euclid(t),
    )
}

fn span(b: &Body) -> (i32, i32, i32, i32) {
    span_xy(b.x, b.y, b.w * SUB, b.h * SUB)
}

fn overlap(a: &Body, b: &Body) -> bool {
    a.x < b.x + b.w * SUB && b.x < a.x + a.w * SUB && a.y < b.y + b.h * SUB && b.y < a.y + a.h * SUB
}

enum VHit {
    None,
    Floor,
    Ceil(i32, i32),
}

/// Muove in orizzontale a passi di 1 px. Ritorna true se urta un muro.
fn step_x(lvl: &Level, b: &mut Body) -> bool {
    let mut dx = b.vx;
    while dx != 0 {
        let s = dx.clamp(-SUB, SUB);
        if lvl.blocked(b.x + s, b.y, b.w * SUB, b.h * SUB) {
            return true;
        }
        b.x += s;
        dx -= s;
    }
    false
}

/// Muove in verticale. Ritorna il tipo di urto (pavimento o soffitto con la tile colpita).
fn step_y(lvl: &Level, b: &mut Body) -> VHit {
    let mut dy = b.vy;
    while dy != 0 {
        let s = dy.clamp(-SUB, SUB);
        if lvl.blocked(b.x, b.y + s, b.w * SUB, b.h * SUB) {
            let t = TILE * SUB;
            return if s > 0 {
                VHit::Floor
            } else {
                VHit::Ceil((b.x + b.w * SUB / 2).div_euclid(t), (b.y + s).div_euclid(t))
            };
        }
        b.y += s;
        dy -= s;
    }
    VHit::None
}

pub struct Game {
    pub state: State,
    pub world: usize,
    pub cursor: usize,
    pub unlocked: usize,
    pub lvl: Level,
    pub p: Player,
    pub mobs: [Mob; MAX_MOBS],
    pub score: u32,
    pub coins: u32,
    pub lives: u8,
    pub cam_x: i32,
    pub clear_t: u32,
    pub frame: u32,
    src: &'static str,
    prev: u8,
}

impl Game {
    /// Nuova partita: mappa del mondo, primo livello sbloccato.
    pub fn new() -> Game {
        let mut g = Game::from_src(levels::LEVELS[0]);
        g.state = State::Map;
        g
    }

    /// Partita su un livello dato dal suo testo (usata anche dai test).
    pub fn from_src(src: &'static str) -> Game {
        let lvl = Level::parse(src).unwrap();
        let p = Player::new(lvl.spawn.0, lvl.spawn.1);
        let mut g = Game {
            state: State::Playing,
            world: 0,
            cursor: 0,
            unlocked: 1,
            lvl,
            p,
            mobs: [NO_MOB; MAX_MOBS],
            score: 0,
            coins: 0,
            lives: LIVES_START,
            cam_x: 0,
            clear_t: 0,
            frame: 0,
            src,
            prev: 0,
        };
        g.spawn_mobs();
        g
    }

    fn load(&mut self, idx: usize) {
        self.world = idx;
        self.src = levels::LEVELS[idx];
        self.clear_t = 0;
        self.restart();
    }

    /// Ricarica il livello corrente da zero (dopo una morte).
    fn restart(&mut self) {
        self.lvl = Level::parse(self.src).unwrap();
        self.p = Player::new(self.lvl.spawn.0, self.lvl.spawn.1);
        self.spawn_mobs();
        self.cam_x = 0;
        self.state = State::Playing;
    }

    fn spawn_mobs(&mut self) {
        self.mobs = [NO_MOB; MAX_MOBS];
        for i in 0..self.lvl.n_mobs {
            let (kind, x, y) = self.lvl.mobs[i];
            self.mobs[i] = Mob {
                kind,
                b: Body {
                    x: x * SUB,
                    y: y * SUB,
                    vx: 0,
                    vy: 0,
                    w: MOB_SIZE,
                    h: MOB_SIZE,
                },
                dir: -1,
                alive: true,
            };
        }
    }

    /// Un passo di simulazione. `input` è una maschera `BTN_*`.
    pub fn update(&mut self, input: u8) {
        let pressed = input & !self.prev;
        self.prev = input;
        self.frame = self.frame.wrapping_add(1);
        match self.state {
            State::Map => self.update_map(pressed),
            State::Playing => self.update_play(input, pressed),
            State::Clear => {
                self.clear_t = self.clear_t.saturating_sub(1);
                if self.clear_t == 0 {
                    if self.world + 1 < levels::LEVELS.len() {
                        self.cursor = self.world + 1;
                        self.state = State::Map;
                    } else {
                        self.state = State::Won;
                    }
                }
            }
            State::Over | State::Won => {
                if pressed & BTN_START != 0 {
                    let prev = self.prev;
                    *self = Game::new();
                    self.prev = prev;
                }
            }
        }
    }

    fn update_map(&mut self, pressed: u8) {
        if pressed & BTN_RIGHT != 0 && self.cursor + 1 < self.unlocked {
            self.cursor += 1;
        }
        if pressed & BTN_LEFT != 0 && self.cursor > 0 {
            self.cursor -= 1;
        }
        if pressed & (BTN_JUMP | BTN_START) != 0 {
            self.load(self.cursor);
        }
    }

    fn update_play(&mut self, input: u8, pressed: u8) {
        let dir = ((input & BTN_RIGHT) != 0) as i32 - ((input & BTN_LEFT) != 0) as i32;
        let max = if input & BTN_RUN != 0 { RUN } else { WALK };
        let p = &mut self.p;
        let acc = if p.on_ground { ACC } else { ACC / 2 };
        if dir != 0 {
            p.b.vx += (dir * max - p.b.vx).clamp(-acc, acc);
            p.facing = dir;
        } else {
            p.b.vx -= p.b.vx.clamp(-FRIC, FRIC);
        }
        if pressed & BTN_JUMP != 0 && p.on_ground {
            p.b.vy = JUMP_V;
        }
        if input & BTN_JUMP == 0 && p.b.vy < JUMP_CUT {
            p.b.vy = JUMP_CUT;
        }
        p.b.vy = (p.b.vy + GRAV).min(MAX_FALL);
        p.on_ground = false;
        let bottom = p.b.y + p.b.h * SUB;

        if step_x(&self.lvl, &mut self.p.b) {
            self.p.b.vx = 0;
        }
        let mut bump = None;
        match step_y(&self.lvl, &mut self.p.b) {
            VHit::Floor => {
                self.p.b.vy = 0;
                self.p.on_ground = true;
            }
            VHit::Ceil(tx, ty) => {
                self.p.b.vy = 0;
                bump = Some((tx, ty));
            }
            VHit::None => {}
        }
        if let Some((tx, ty)) = bump {
            self.bump(tx, ty);
        }
        if self.p.b.y > (self.lvl.h as i32 * TILE + TILE) * SUB {
            self.die();
            return;
        }
        self.touch_tiles();
        if self.state != State::Playing {
            return;
        }
        self.update_mobs();
        self.touch_mobs(bottom);
        if self.state != State::Playing {
            return;
        }
        self.p.star = self.p.star.saturating_sub(1);
        self.p.hurt = self.p.hurt.saturating_sub(1);
        let max_cam = (self.lvl.w as i32 * TILE - VIEW_W).max(0);
        let target = self.p.b.x / SUB + self.p.b.w / 2 - VIEW_W / 2;
        self.cam_x = target.clamp(0, max_cam);
    }

    fn bump(&mut self, tx: i32, ty: i32) {
        let (ux, uy) = (tx as usize, ty as usize);
        match self.lvl.tile(tx, ty) {
            Tile::Question(r) => {
                self.lvl.tiles[uy][ux] = Tile::Used;
                match r {
                    Reward::Coin => self.add_coin(),
                    Reward::Mushroom => self.spawn_item(MobKind::Mushroom, tx, ty),
                    Reward::Star => self.spawn_item(MobKind::Star, tx, ty),
                }
            }
            Tile::Brick if self.p.big => {
                self.lvl.tiles[uy][ux] = Tile::Air;
                self.score += 50;
            }
            _ => {}
        }
    }

    fn spawn_item(&mut self, kind: MobKind, tx: i32, ty: i32) {
        if let Some(m) = self.mobs.iter_mut().find(|m| !m.alive) {
            *m = Mob {
                kind,
                b: Body {
                    x: tx * TILE * SUB,
                    y: (ty - 1) * TILE * SUB,
                    vx: 0,
                    vy: 0,
                    w: MOB_SIZE,
                    h: MOB_SIZE,
                },
                dir: 1,
                alive: true,
            };
        }
    }

    fn add_coin(&mut self) {
        self.coins += 1;
        self.score += 100;
        if self.coins % 100 == 0 {
            self.lives = self.lives.saturating_add(1);
        }
    }

    fn touch_tiles(&mut self) {
        let (x0, x1, y0, y1) = span(&self.p.b);
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                match self.lvl.tile(tx, ty) {
                    Tile::Coin => {
                        self.lvl.tiles[ty as usize][tx as usize] = Tile::Air;
                        self.add_coin();
                    }
                    Tile::Flag => {
                        self.level_clear();
                        return;
                    }
                    _ => {}
                }
            }
        }
    }

    /// Forza la fine del livello (solo test).
    pub fn level_clear_for_test(&mut self) {
        self.level_clear();
    }

    fn level_clear(&mut self) {
        self.score += 1000;
        self.unlocked = (self.world + 2)
            .min(levels::LEVELS.len())
            .max(self.unlocked);
        self.state = State::Clear;
        self.clear_t = CLEAR_FRAMES;
    }

    fn update_mobs(&mut self) {
        for m in self.mobs.iter_mut().filter(|m| m.alive) {
            let speed = if m.kind == MobKind::Star {
                STAR_SPEED
            } else {
                MOB_SPEED
            };
            m.b.vx = m.dir * speed;
            m.b.vy = (m.b.vy + GRAV).min(MAX_FALL);
            if step_x(&self.lvl, &mut m.b) {
                m.dir = -m.dir;
            }
            if !matches!(step_y(&self.lvl, &mut m.b), VHit::None) {
                m.b.vy = 0;
            }
            if m.b.y > (self.lvl.h as i32 * TILE + TILE) * SUB {
                m.alive = false;
            }
        }
    }

    fn touch_mobs(&mut self, prev_bottom: i32) {
        for i in 0..MAX_MOBS {
            let m = self.mobs[i];
            if !m.alive || !overlap(&self.p.b, &m.b) {
                continue;
            }
            match m.kind {
                MobKind::Mushroom => {
                    self.mobs[i].alive = false;
                    if self.p.big {
                        self.score += 1000;
                    } else {
                        self.set_big(true);
                    }
                }
                MobKind::Star => {
                    self.mobs[i].alive = false;
                    self.p.star = STAR_FRAMES;
                    self.score += 1000;
                }
                MobKind::Fungo => {
                    if self.p.star > 0 {
                        self.kill(i, 200);
                    } else if self.p.b.vy > 0 && prev_bottom <= m.b.y + 4 * SUB {
                        self.kill(i, 100);
                        self.p.b.vy = JUMP_V / 2;
                    } else if self.p.hurt == 0 {
                        self.hurt();
                        return;
                    }
                }
            }
        }
    }

    fn kill(&mut self, i: usize, pts: u32) {
        self.mobs[i].alive = false;
        self.score += pts;
    }

    fn set_big(&mut self, big: bool) {
        if self.p.big == big {
            return;
        }
        self.p.big = big;
        let d = 10 * SUB;
        if big {
            // Non cresce se il soffitto è basso: resterebbe incastrato nel tile.
            if self.lvl.blocked(self.p.b.x, self.p.b.y - d, self.p.b.w * SUB, 24 * SUB) {
                return;
            }
            self.p.b.y -= d;
            self.p.b.h = 24;
        } else {
            self.p.b.y += d;
            self.p.b.h = 14;
        }
    }

    fn hurt(&mut self) {
        if self.p.big {
            self.set_big(false);
            self.p.hurt = HURT_FRAMES;
        } else {
            self.die();
        }
    }

    fn die(&mut self) {
        self.lives = self.lives.saturating_sub(1);
        if self.lives == 0 {
            self.state = State::Over;
        } else {
            self.restart();
        }
    }
}
