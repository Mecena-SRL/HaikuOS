//! Wilkanoid: logica di gioco a punto fisso, `no_std`, senza allocazione.
//! Campo logico 240x135 px (16:9). 1 px = `SUB` subunità.

#![cfg_attr(not(test), no_std)]

pub mod render;

pub const SUB: i32 = 16;
pub const PW: i32 = 240;
pub const PH: i32 = 135;
pub const COLS: usize = 9;
pub const ROWS: usize = 6;
pub const BW: i32 = 24;
pub const BH: i32 = 7;
pub const BGX: i32 = 2;
pub const BGY: i32 = 3;
pub const BX0: i32 = 4;
pub const BY0: i32 = 20;
pub const PAD_Y: i32 = 124;
pub const PAD_H: i32 = 3;
pub const PAD_W: i32 = 28;
pub const PAD_WIDE: i32 = 44;
pub const BALL_R: i32 = 24; // 1.5 px
pub const LIVES: u8 = 3;
/// Posizioni precedenti della palla per la scia.
pub const TRAIL: usize = 5;
pub const PARTS: usize = 32;
const PAD_STEP: i32 = 3 * SUB;
const VY: i32 = 20;
const VX_MAX: i32 = 18;
const VX_MIN: i32 = 6; // evita rimbalzi verticali infiniti
const DROP_V: i32 = 8;
const WIDE_TICKS: u32 = 600;
const LOST_TICKS: u32 = 60;

pub const EMPTY: u8 = 0;
pub const NORMAL: u8 = 1;
pub const STEEL: u8 = 2;
pub const CRACKED: u8 = 3;

/// Livelli: '0' vuoto, '1' mattone normale, '2' acciaio (due colpi).
pub const LEVELS: [[&[u8; COLS]; ROWS]; 4] = [
    [b"111111111", b"111111111", b"111111111", b"111111111", b"111111111", b"111111111"],
    [b"000010000", b"000111000", b"001111100", b"011111110", b"111111111", b"101010101"],
    [b"222022022", b"111111111", b"202020202", b"111111111", b"222222222", b"010101010"],
    [b"000020000", b"000121000", b"001212100", b"012121210", b"001212100", b"000121000"],
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Title,
    Ready,
    Play,
    Lost,
    Won,
    Over,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd {
    Left,
    Right,
    Fire,
}

/// Particella di distruzione (subunità). `tone` = indice riga, 6 = acciaio.
#[derive(Clone, Copy, Default)]
pub struct Part {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub life: u32,
    pub tone: u8,
}

#[derive(Clone, Copy)]
pub struct Game {
    pub phase: Phase,
    pub level: usize,
    pub score: u32,
    pub lives: u8,
    /// Centro della racchetta, subunità.
    pub paddle: i32,
    /// Tick rimanenti della racchetta larga.
    pub wide: u32,
    /// Centro della palla, subunità.
    pub ball: (i32, i32),
    /// Velocità della palla, subunità per tick (y negativo = su).
    pub vel: (i32, i32),
    pub cells: [[u8; COLS]; ROWS],
    /// Power-up che cadono.
    pub drops: [Option<(i32, i32)>; 4],
    pub trail: [(i32, i32); TRAIL],
    pub parts: [Part; PARTS],
    timer: u32,
    rng: u32,
}

impl Game {
    /// Nuova partita in fase `Title`, livello 1 caricato sullo sfondo.
    pub fn new(seed: u32) -> Self {
        let mut g = Game {
            phase: Phase::Title,
            level: 0,
            score: 0,
            lives: LIVES,
            paddle: PW * SUB / 2,
            wide: 0,
            ball: (0, 0),
            vel: (0, 0),
            cells: [[EMPTY; COLS]; ROWS],
            drops: [None; 4],
            trail: [(0, 0); TRAIL],
            parts: [Part::default(); PARTS],
            timer: 0,
            rng: seed | 1,
        };
        g.load(0);
        g
    }

    /// Partita pronta (fase `Ready`) dal livello `l`.
    pub fn at_level(seed: u32, l: usize) -> Self {
        let mut g = Self::new(seed);
        g.start_at(l.min(LEVELS.len() - 1));
        g
    }

    /// Racchetta: metà larghezza in subunità.
    pub fn half(&self) -> i32 {
        (if self.wide > 0 { PAD_WIDE } else { PAD_W }) * SUB / 2
    }

    pub fn bricks_left(&self) -> usize {
        self.cells.iter().flatten().filter(|&&c| c != EMPTY).count()
    }

    pub fn input(&mut self, c: Cmd) {
        match self.phase {
            Phase::Title | Phase::Over | Phase::Won => {
                if c == Cmd::Fire {
                    self.start_at(0);
                }
            }
            Phase::Ready | Phase::Play => match c {
                Cmd::Left => self.move_paddle(-PAD_STEP),
                Cmd::Right => self.move_paddle(PAD_STEP),
                Cmd::Fire => {
                    if self.phase == Phase::Ready {
                        self.phase = Phase::Play;
                        self.vel = (VX_MAX / 2, -VY);
                    }
                }
            },
            Phase::Lost => {}
        }
    }

    pub fn tick(&mut self) {
        match self.phase {
            Phase::Ready => self.ball.0 = self.paddle,
            Phase::Lost => {
                self.timer = self.timer.saturating_sub(1);
                if self.timer == 0 {
                    self.phase = Phase::Ready;
                }
            }
            Phase::Play => self.play(),
            _ => {}
        }
    }

    fn start_at(&mut self, l: usize) {
        self.score = 0;
        self.lives = LIVES;
        self.phase = Phase::Ready;
        self.load(l);
    }

    fn load(&mut self, l: usize) {
        self.level = l;
        for r in 0..ROWS {
            for c in 0..COLS {
                self.cells[r][c] = LEVELS[l][r][c] - b'0';
            }
        }
        self.wide = 0;
        self.drops = [None; 4];
        self.parts = [Part::default(); PARTS];
        self.reset_ball();
    }

    fn reset_ball(&mut self) {
        self.ball = (self.paddle, PAD_Y * SUB - BALL_R);
        self.vel = (0, 0);
        self.trail = [self.ball; TRAIL];
    }

    fn move_paddle(&mut self, d: i32) {
        let h = self.half();
        let p = self.paddle + d;
        self.paddle = if p < h {
            h
        } else if p > PW * SUB - h {
            PW * SUB - h
        } else {
            p
        };
    }

    fn rand(&mut self) -> u32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        x
    }

    fn burst(&mut self, x: i32, y: i32, tone: u8) {
        for _ in 0..6 {
            let vx = (self.rand() % 41) as i32 - 20;
            let vy = -((self.rand() % 20) as i32) - 6;
            let life = 18 + self.rand() % 12;
            if let Some(p) = self.parts.iter_mut().find(|p| p.life == 0) {
                *p = Part { x, y, vx, vy, life, tone };
            }
        }
    }

    fn play(&mut self) {
        if self.wide > 0 {
            self.wide -= 1;
        }
        let (px, py) = self.ball;
        self.trail.copy_within(0..TRAIL - 1, 1);
        self.trail[0] = self.ball;
        for p in self.parts.iter_mut() {
            if p.life > 0 {
                p.x += p.vx;
                p.y += p.vy;
                p.vy += 2;
                p.life -= 1;
            }
        }
        let (mut x, mut y) = (px + self.vel.0, py + self.vel.1);
        let (mut vx, mut vy) = self.vel;
        let top = PAD_Y * SUB;

        // Pareti.
        if x < BALL_R {
            x = BALL_R;
            vx = vx.abs();
        }
        if x > PW * SUB - BALL_R {
            x = PW * SUB - BALL_R;
            vx = -vx.abs();
        }
        if y < BALL_R {
            y = BALL_R;
            vy = vy.abs();
        }

        // Racchetta: l'angolo dipende da dove tocca.
        let half = self.half();
        if vy > 0
            && y + BALL_R >= top
            && y - BALL_R <= top + PAD_H * SUB
            && x >= self.paddle - half
            && x <= self.paddle + half
        {
            y = top - BALL_R;
            vy = -VY;
            vx = ((x - self.paddle) * VX_MAX / half).clamp(-VX_MAX, VX_MAX);
            if vx.abs() < VX_MIN {
                vx = if x < self.paddle { -VX_MIN } else { VX_MIN };
            }
        }

        // Mattoni: al primo colpo si rimbalza e si esce.
        'bricks: for r in 0..ROWS {
            for c in 0..COLS {
                if self.cells[r][c] == EMPTY {
                    continue;
                }
                let bx0 = (BX0 + c as i32 * (BW + BGX)) * SUB;
                let by0 = (BY0 + r as i32 * (BH + BGY)) * SUB;
                let (bx1, by1) = (bx0 + BW * SUB, by0 + BH * SUB);
                if x < bx0 - BALL_R || x > bx1 + BALL_R || y < by0 - BALL_R || y > by1 + BALL_R {
                    continue;
                }
                let from_above_below = !(py >= by0 && py <= by1);
                let from_side = !(px >= bx0 && px <= bx1);
                if from_above_below || !from_side {
                    let up = py < by0;
                    vy = if up { -vy.abs() } else { vy.abs() };
                    y = if up { by0 - BALL_R } else { by1 + BALL_R };
                }
                if from_side {
                    let left = px < bx0;
                    vx = if left { -vx.abs() } else { vx.abs() };
                    x = if left { bx0 - BALL_R } else { bx1 + BALL_R };
                }
                let cell = self.cells[r][c];
                let pts = (self.level as u32 + 1) * if cell == NORMAL { 10 } else if cell == STEEL { 5 } else { 20 };
                self.cells[r][c] = match cell {
                    STEEL => CRACKED,
                    _ => EMPTY,
                };
                self.score += pts;
                let tone = if cell == NORMAL { r as u8 } else { 6 };
                self.burst(bx0 + BW / 2 * SUB, by0 + BH / 2 * SUB, tone);
                if self.cells[r][c] == EMPTY && self.rand() % 5 == 0 {
                    if let Some(slot) = self.drops.iter_mut().find(|d| d.is_none()) {
                        *slot = Some((bx0 + BW / 2 * SUB, by0));
                    }
                }
                break 'bricks;
            }
        }
        self.ball = (x, y);
        self.vel = (vx, vy);

        if self.bricks_left() == 0 {
            if self.level + 1 < LEVELS.len() {
                self.load(self.level + 1);
                self.phase = Phase::Ready;
            } else {
                self.phase = Phase::Won;
            }
            return;
        }

        // Power-up: cadono, la racchetta li prende.
        for i in 0..self.drops.len() {
            if let Some((dx, dy)) = self.drops[i] {
                let ny = dy + DROP_V;
                if ny >= top && ny <= top + (PAD_H + 2) * SUB && (dx - self.paddle).abs() <= self.half() + 4 * SUB {
                    self.wide = WIDE_TICKS;
                    self.drops[i] = None;
                    self.move_paddle(0); // la racchetta larga non deve sporgere dal campo
                } else if ny > PH * SUB {
                    self.drops[i] = None;
                } else {
                    self.drops[i] = Some((dx, ny));
                }
            }
        }

        // Palla persa.
        if y - BALL_R > PH * SUB {
            self.lives -= 1;
            self.wide = 0;
            self.drops = [None; 4];
            self.reset_ball();
            if self.lives == 0 {
                self.phase = Phase::Over;
            } else {
                self.phase = Phase::Lost;
                self.timer = LOST_TICKS;
            }
        }
    }
}

/// Giocatore automatico: segue la palla e lancia. Serve per i test e gli screenshot.
pub fn autoplay(g: &mut Game) {
    match g.phase {
        Phase::Title | Phase::Over | Phase::Won | Phase::Ready => g.input(Cmd::Fire),
        Phase::Play => {
            let d = g.ball.0 - g.paddle;
            if d < -PAD_STEP {
                g.input(Cmd::Left);
            } else if d > PAD_STEP {
                g.input(Cmd::Right);
            }
        }
        Phase::Lost => {}
    }
    g.tick();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bot_clears_all_levels() {
        let mut g = Game::new(7);
        g.input(Cmd::Fire);
        for _ in 0..600_000 {
            if matches!(g.phase, Phase::Won | Phase::Over) {
                break;
            }
            autoplay(&mut g);
        }
        assert_eq!(g.phase, Phase::Won);
        assert_eq!(g.level, LEVELS.len() - 1);
        assert!(g.score > 0);
    }

    #[test]
    fn without_paddle_all_lives_are_lost() {
        let mut g = Game::new(3);
        g.input(Cmd::Fire);
        for _ in 0..200_000 {
            if g.phase == Phase::Over {
                break;
            }
            if g.phase == Phase::Ready {
                g.input(Cmd::Fire);
            }
            g.tick();
        }
        assert_eq!(g.phase, Phase::Over);
        assert_eq!(g.lives, 0);
    }

    #[test]
    fn ball_bounces_off_side_wall() {
        let mut g = Game::at_level(1, 0);
        g.phase = Phase::Play;
        g.ball = (BALL_R + 1, 100 * SUB);
        g.vel = (-VY, 0);
        g.tick();
        assert!(g.vel.0 > 0);
    }
}
