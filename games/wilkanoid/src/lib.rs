//! Wilkanoid: logica a punto fisso, `no_std`, senza allocazione.
//! Campo logico 240x135 px (16:9). 1 px = `SUB` subunità.
//! Collisioni palla-mattone con cerchio contro rettangolo (punto più vicino, senza sqrt).

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
/// Raggio della palla: 1.5 px.
pub const BALL_R: i32 = 24;
pub const LIVES: u8 = 3;
pub const MAX_BALLS: usize = 3;
pub const TRAIL: usize = 5;
pub const PARTS: usize = 32;
/// Velocità verticale base (subunità per tick); cresce di 1 per livello.
const VY: i32 = 20;
/// Componente orizzontale massima e minima dopo un colpo sulla racchetta.
const VX_MAX: i32 = 18;
const VX_MIN: i32 = 6;
const DROP_V: i32 = 8;
const WIDE_TICKS: u32 = 600;
const LOST_TICKS: u32 = 60;
const FLASH_TICKS: u8 = 6;

pub const EMPTY: u8 = 0;
pub const NORMAL: u8 = 1;
pub const STEEL: u8 = 2;
pub const CRACKED: u8 = 3;
/// Mattone esplosivo: distrugge i vicini quando cade.
pub const BOMB: u8 = 4;
/// Tono delle particelle: indice riga per i mattoni normali, poi acciaio ed esplosivo.
pub const TONE_STEEL: u8 = 6;
pub const TONE_BOMB: u8 = 7;

/// Power-up che cadono.
pub const DROP_WIDE: u8 = 0;
pub const DROP_MULTI: u8 = 1;

/// Livelli: '0' vuoto, '1' normale, '2' acciaio (due colpi), '4' esplosivo.
pub const LEVELS: [[&[u8; COLS]; ROWS]; 6] = [
    [b"111111111", b"111111111", b"111111111", b"121111121", b"111111111", b"111111111"],
    [b"000020000", b"000111000", b"001111100", b"011111110", b"111111111", b"101010101"],
    [b"222022022", b"111111111", b"202020202", b"111111111", b"222222222", b"010101010"],
    [b"000020000", b"000222000", b"002222200", b"022222220", b"222222222", b"022222220"],
    [b"111010111", b"011414110", b"110111011", b"111010111", b"011111110", b"111111111"],
    [b"222222222", b"212121212", b"222222222", b"141212141", b"222222222", b"010101010"],
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

/// Palla, in subunità (posizione del centro, velocità per tick; y negativo = su).
#[derive(Clone, Copy, Default, Debug)]
pub struct Ball {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
}

/// Particella di distruzione. `tone` = indice riga, `TONE_STEEL` o `TONE_BOMB`.
#[derive(Clone, Copy, Default)]
pub struct Part {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub life: u32,
    pub tone: u8,
}

/// Power-up in caduta.
#[derive(Clone, Copy)]
pub struct Drop {
    pub x: i32,
    pub y: i32,
    pub kind: u8,
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
    pub balls: [Option<Ball>; MAX_BALLS],
    pub cells: [[u8; COLS]; ROWS],
    /// Tick di lampo residuo per ogni mattone colpito.
    pub flash: [[u8; COLS]; ROWS],
    pub drops: [Option<Drop>; 4],
    pub parts: [Part; PARTS],
    pub trail: [(i32, i32); TRAIL],
    /// Tick di scossa residua dello schermo.
    pub shake: u32,
    /// Fire premuto durante la perdita di vita: lancia alla fine della pausa.
    pub launch: bool,
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
            balls: [None; MAX_BALLS],
            cells: [[EMPTY; COLS]; ROWS],
            flash: [[0; COLS]; ROWS],
            drops: [None; 4],
            parts: [Part::default(); PARTS],
            trail: [(0, 0); TRAIL],
            shake: 0,
            launch: false,
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

    /// Palla di riferimento (la prima in gioco).
    pub fn lead(&self) -> Option<Ball> {
        self.balls.iter().flatten().next().copied()
    }

    /// Velocità verticale del livello corrente.
    pub fn speed(&self) -> i32 {
        VY + self.level as i32
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
                Cmd::Left => self.move_paddle(-3 * SUB),
                Cmd::Right => self.move_paddle(3 * SUB),
                Cmd::Fire => {
                    if self.phase == Phase::Ready {
                        self.launch_ball();
                    }
                }
            },
            Phase::Lost => {
                if c == Cmd::Fire {
                    self.launch = true;
                }
            }
        }
    }

    pub fn tick(&mut self) {
        match self.phase {
            Phase::Ready => {
                if let Some(b) = self.balls[0].as_mut() {
                    b.x = self.paddle;
                }
            }
            Phase::Lost => {
                self.timer = self.timer.saturating_sub(1);
                if self.timer == 0 {
                    self.phase = Phase::Ready;
                    if self.launch {
                        self.launch = false;
                        self.launch_ball();
                    }
                }
            }
            Phase::Play => self.play(),
            _ => {}
        }
    }

    fn launch_ball(&mut self) {
        let vy = -self.speed();
        if let Some(b) = self.balls[0].as_mut() {
            b.vx = VX_MAX / 2;
            b.vy = vy;
        }
        self.phase = Phase::Play;
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
        self.flash = [[0; COLS]; ROWS];
        self.wide = 0;
        self.drops = [None; 4];
        self.parts = [Part::default(); PARTS];
        self.reset_ball();
    }

    fn reset_ball(&mut self) {
        let b = Ball { x: self.paddle, y: PAD_Y * SUB - BALL_R, vx: 0, vy: 0 };
        self.balls = [None; MAX_BALLS];
        self.balls[0] = Some(b);
        self.trail = [(b.x, b.y); TRAIL];
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
        self.shake = self.shake.saturating_sub(1);
        for r in 0..ROWS {
            for c in 0..COLS {
                self.flash[r][c] = self.flash[r][c].saturating_sub(1);
            }
        }
        for p in self.parts.iter_mut().filter(|p| p.life > 0) {
            p.x += p.vx;
            p.y += p.vy;
            p.vy += 2;
            p.life -= 1;
        }
        if let Some(b) = self.lead() {
            self.trail.copy_within(0..TRAIL - 1, 1);
            self.trail[0] = (b.x, b.y);
        }

        for i in 0..MAX_BALLS {
            if let Some(b) = self.balls[i] {
                self.balls[i] = self.step_ball(b);
            }
        }

        if self.balls.iter().all(|b| b.is_none()) {
            self.lives -= 1;
            self.wide = 0;
            self.drops = [None; 4];
            self.shake = 16;
            self.reset_ball();
            if self.lives == 0 {
                self.phase = Phase::Over;
            } else {
                self.phase = Phase::Lost;
                self.timer = LOST_TICKS;
            }
            return;
        }

        if self.bricks_left() == 0 {
            if self.level + 1 < LEVELS.len() {
                self.load(self.level + 1);
                self.phase = Phase::Ready;
            } else {
                self.phase = Phase::Won;
            }
            return;
        }

        self.update_drops();
    }

    /// Un passo di una palla: muri, racchetta, mattoni. `None` se è uscita dal campo.
    fn step_ball(&mut self, b: Ball) -> Option<Ball> {
        let (px, py) = (b.x, b.y);
        let (mut x, mut y, mut vx, mut vy) = (px + b.vx, py + b.vy, b.vx, b.vy);
        let vx_in = b.vx;
        let top = PAD_Y * SUB;

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
            vy = -self.speed();
            vx = ((x - self.paddle) * VX_MAX / half).clamp(-VX_MAX, VX_MAX);
            if vx.abs() < VX_MIN {
                // Colpo al centro: angolo casuale, così la palla non entra in un'orbita periodica.
                let side = if x == self.paddle { vx_in.signum() } else if x < self.paddle { -1 } else { 1 };
                let side = if side == 0 { 1 } else { side };
                vx = side * (VX_MIN + (self.rand() % 6) as i32);
            }
        }

        // Mattoni: il primo colpo rimbalza; gli altri mattoni toccati nello stesso tick si rompono senza rimbalzo.
        let mut bounced = false;
        let mut boom: Option<(usize, usize)> = None;
        for r in 0..ROWS {
            for c in 0..COLS {
                if self.cells[r][c] == EMPTY {
                    continue;
                }
                let bx0 = (BX0 + c as i32 * (BW + BGX)) * SUB;
                let by0 = (BY0 + r as i32 * (BH + BGY)) * SUB;
                let (bx1, by1) = (bx0 + BW * SUB, by0 + BH * SUB);
                // Punto del rettangolo più vicino al centro della palla.
                let (cx, cy) = (x.clamp(bx0, bx1), y.clamp(by0, by1));
                let (dx, dy) = (x - cx, y - cy);
                if dx * dx + dy * dy > BALL_R * BALL_R {
                    continue;
                }
                if !bounced {
                    // Dal sopra/sotto: rimbalzo verticale. Dal lato: orizzontale. Angolo: asse da cui arrivava la palla.
                    let from_above_below = !(py >= by0 && py <= by1);
                    let from_side = !(px >= bx0 && px <= bx1);
                    let vertical = from_above_below || !from_side;
                    let horizontal = from_side && !from_above_below;
                    if vertical {
                        let up = py < by0;
                        vy = if up { -vy.abs() } else { vy.abs() };
                        y = if up { by0 - BALL_R } else { by1 + BALL_R };
                    }
                    if horizontal {
                        let left = px < bx0;
                        vx = if left { -vx.abs() } else { vx.abs() };
                        x = if left { bx0 - BALL_R } else { bx1 + BALL_R };
                    }
                    bounced = true;
                }
                let cell = self.cells[r][c];
                if cell == BOMB {
                    boom = Some((r, c));
                }
                let pts = (self.level as u32 + 1)
                    * match cell {
                        NORMAL => 10,
                        STEEL => 5,
                        BOMB => 15,
                        _ => 20,
                    };
                self.cells[r][c] = match cell {
                    STEEL => CRACKED,
                    _ => EMPTY,
                };
                self.score += pts;
                self.flash[r][c] = FLASH_TICKS;
                self.shake = self.shake.max(4);
                let tone = match cell {
                    NORMAL => r as u8,
                    BOMB => TONE_BOMB,
                    _ => TONE_STEEL,
                };
                let (mx, my) = (bx0 + BW / 2 * SUB, by0 + BH / 2 * SUB);
                self.burst(mx, my, tone);
                if self.cells[r][c] == EMPTY && self.rand() % 5 == 0 {
                    let kind = if self.rand() % 3 == 0 { DROP_MULTI } else { DROP_WIDE };
                    if let Some(slot) = self.drops.iter_mut().find(|d| d.is_none()) {
                        *slot = Some(Drop { x: mx, y: by0, kind });
                    }
                }
            }
        }

        if let Some((r, c)) = boom {
            self.explode(r, c);
        }

        // Palla persa: esce sotto il campo.
        if y - BALL_R > PH * SUB {
            return None;
        }
        Some(Ball { x, y, vx, vy })
    }

    /// Esplosione: rompe i mattoni vicini (non l'esplosivo stesso, già rotto).
    fn explode(&mut self, r: usize, c: usize) {
        for rr in r.saturating_sub(1)..=(r + 1).min(ROWS - 1) {
            for cc in c.saturating_sub(1)..=(c + 1).min(COLS - 1) {
                if (rr, cc) == (r, c) || self.cells[rr][cc] == EMPTY {
                    continue;
                }
                self.score += (self.level as u32 + 1) * 10;
                self.cells[rr][cc] = EMPTY;
                self.flash[rr][cc] = FLASH_TICKS;
                let (mx, my) = ((BX0 + cc as i32 * (BW + BGX)) * SUB + BW / 2 * SUB, (BY0 + rr as i32 * (BH + BGY)) * SUB + BH / 2 * SUB);
                self.burst(mx, my, TONE_BOMB);
            }
        }
        self.shake = self.shake.max(10);
    }

    /// Power-up: cadono e la racchetta li prende.
    fn update_drops(&mut self) {
        let top = PAD_Y * SUB;
        for i in 0..self.drops.len() {
            if let Some(d) = self.drops[i] {
                let ny = d.y + DROP_V;
                if ny >= top && ny <= top + (PAD_H + 2) * SUB && (d.x - self.paddle).abs() <= self.half() + 4 * SUB {
                    self.drops[i] = None;
                    if d.kind == DROP_MULTI {
                        self.spawn_extra_balls();
                    } else {
                        self.wide = WIDE_TICKS;
                        self.move_paddle(0); // la racchetta larga non deve sporgere dal campo
                    }
                } else if ny > PH * SUB {
                    self.drops[i] = None;
                } else {
                    self.drops[i] = Some(Drop { y: ny, ..d });
                }
            }
        }
    }

    /// Multipalla: copia la palla di riferimento in due direzioni, fino a `MAX_BALLS`.
    fn spawn_extra_balls(&mut self) {
        let Some(b) = self.lead() else { return };
        for vx in [-VX_MAX / 2, VX_MAX / 2] {
            if let Some(slot) = self.balls.iter_mut().find(|s| s.is_none()) {
                *slot = Some(Ball { vx, ..b });
            }
        }
    }
}

/// Giocatore automatico: segue la palla di riferimento e lancia. Serve per i test e gli screenshot.
pub fn autoplay(g: &mut Game) {
    match g.phase {
        Phase::Title | Phase::Over | Phase::Won | Phase::Ready => g.input(Cmd::Fire),
        Phase::Play => {
            let target = g.lead().map_or(g.paddle, |b| b.x);
            let d = target - g.paddle;
            if d < -3 * SUB {
                g.input(Cmd::Left);
            } else if d > 3 * SUB {
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
        for _ in 0..900_000 {
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
        g.balls = [None; MAX_BALLS];
        g.balls[0] = Some(Ball { x: BALL_R + 1, y: 100 * SUB, vx: -VY, vy: 0 });
        g.tick();
        assert!(g.lead().map_or(false, |b| b.vx > 0));
    }
}

#[cfg(test)]
mod seed_sweep {
    use super::*;

    #[test]
    fn bot_wins_many_seeds() {
        for seed in 1..=40u32 {
            let mut g = Game::new(seed);
            g.input(Cmd::Fire);
            for _ in 0..400_000 {
                if matches!(g.phase, Phase::Won | Phase::Over) {
                    break;
                }
                autoplay(&mut g);
            }
            assert_eq!(g.phase, Phase::Won, "seed {seed} bloccato al livello {}", g.level + 1);
        }
    }
}
