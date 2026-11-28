//! Core del pinball: fisica a virgola fissa (1 px = 256), punteggio, stati.
//! `no_std`, nessuna allocazione. Il rendering sta fuori (vedi `main.rs`).

#![cfg_attr(not(test), no_std)]

pub type V = (i64, i64);

pub const FP: i64 = 256;
pub const TABLE_W: i64 = 160;
pub const TABLE_H: i64 = 240;
pub const MAX_BALLS: usize = 3;
/// Centri dei paraurti in px.
pub const BUMPERS: [V; 4] = [(40, 90), (75, 75), (105, 100), (70, 120)];

pub const R: i64 = 3 * FP; // raggio pallina
pub const BUMPER_R: i64 = 8 * FP;
pub const SENSOR_R: i64 = 6 * FP;
pub const SENSORS: [V; 3] = [(30, 30), (60, 22), (90, 30)];
const SENSOR_ALL: u8 = 0b111;
const SUB: i64 = 8; // sottopassi per tick: niente tunneling
const GRAVITY: i64 = 13; // ~0.05 px/tick²
const VMAX: i64 = 16 * FP;
const E_WALL: i64 = 154; // restituzione 0.6
const E_BUMPER: i64 = 230; // 0.9
const E_KICK: i64 = 200; // 0.8
const KICK: i64 = 6 * FP;
const FLIP_LEN: i64 = 28 * FP;
const FLIP_SPEED: i64 = 20; // gradi per tick
const LANE: V = (149 * FP, 215 * FP);
const MULTI_AT: u32 = 5000;
pub const WALLS: [[i64; 4]; 7] = [
    [1, 1, 159, 1],       // soffitto
    [1, 1, 1, 200],       // sinistra
    [159, 1, 159, 232],   // destra
    [1, 200, 48, 232],    // uscita sinistra
    [138, 200, 116, 232], // uscita destra
    [138, 44, 138, 240],  // divisorio corsia
    [138, 14, 156, 40],   // guida corsia -> sinistra
];
pub const KICKERS: [[i64; 4]; 2] = [[30, 170, 48, 200], [112, 170, 128, 198]];
/// Seno in gradi 0..=90, scala FP.
const SIN: [i64; 91] = [
    0, 4, 9, 13, 18, 22, 27, 31, 36, 40, 44, 49, 53, 58, 62, 66, 71, 75, 79, 83, 88, 92, 96, 100,
    104, 108, 112, 116, 120, 124, 128, 132, 136, 139, 143, 147, 150, 154, 158, 161, 165, 168, 171,
    175, 178, 181, 184, 187, 190, 193, 196, 199, 202, 204, 207, 210, 212, 215, 217, 219, 222, 224,
    226, 228, 230, 232, 234, 236, 237, 239, 241, 242, 243, 245, 246, 247, 248, 249, 250, 251, 252,
    253, 254, 254, 255, 255, 255, 256, 256, 256, 256,
];

#[derive(Clone, Copy, Default, Debug)]
pub struct Ball {
    pub pos: V,
    pub vel: V,
    pub live: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Flip {
    pub pivot: V,
    pub ang: i64,
    pub dps: i64,
    rest: i64,
    up: i64,
}

impl Flip {
    /// Punta del flipper in FP.
    pub fn tip(&self) -> V {
        self.tip_at(self.ang)
    }

    /// Punta alla rotazione `ang` (per sottopasso).
    pub fn tip_at(&self, ang: i64) -> V {
        let (c, s) = sin_cos(ang);
        (self.pivot.0 + FLIP_LEN * c / FP, self.pivot.1 + FLIP_LEN * s / FP)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Ready,
    Playing,
    Over,
}

#[derive(Clone, Copy, Default)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub plunger: bool,
}

pub struct Pinball {
    pub balls: [Ball; MAX_BALLS],
    pub flips: [Flip; 2],
    pub state: State,
    pub score: u32,
    pub high: u32,
    pub lives: u8,
    pub mult: u32,
    pub lit: u8,
    pub charge: i64,
    multi_done: bool,
    prev_plunger: bool,
    ball_pts: u32,
    /// Mondo corrente: 0 porto, 1 mare aperto, 2 sala del faro.
    pub world: u8,
    /// Passo della missione della guardiana (0..3), sensori nell'ordine SENSORS.
    pub mission: u8,
    /// Sensori in cui la pallina si trova ora (rilevamento per ingresso).
    inside: u8,
}

impl Pinball {
    pub fn new() -> Self {
        let mut p = Pinball {
            balls: [Ball::default(); MAX_BALLS],
            flips: [
                Flip { pivot: (50 * FP, 220 * FP), ang: 25, dps: 0, rest: 25, up: -30 },
                Flip { pivot: (110 * FP, 220 * FP), ang: 155, dps: 0, rest: 155, up: 210 },
            ],
            state: State::Ready,
            score: 0,
            high: 0,
            lives: 3,
            mult: 1,
            lit: 0,
            charge: 0,
            multi_done: false,
            prev_plunger: false,
            ball_pts: 0,
            world: 0,
            mission: 0,
            inside: 0,
        };
        p.reset();
        p
    }

    /// Nuova partita; il record resta.
    pub fn reset(&mut self) {
        self.score = 0;
        self.lives = 3;
        self.mult = 1;
        self.lit = 0;
        self.charge = 0;
        self.multi_done = false;
        self.prev_plunger = false;
        self.ball_pts = 0;
        self.world = 0;
        self.mission = 0;
        self.inside = 0;
        self.state = State::Ready;
        self.balls = [Ball::default(); MAX_BALLS];
        self.balls[0] = Ball { pos: LANE, vel: (0, 0), live: true };
        for f in self.flips.iter_mut() {
            f.ang = f.rest;
            f.dps = 0;
        }
    }

    pub fn tick(&mut self, i: Input) {
        if self.state == State::Over {
            return;
        }
        self.flippers(i);
        match self.state {
            State::Ready => {
                if i.plunger {
                    self.charge = (self.charge + 2).min(100);
                } else if self.prev_plunger {
                    self.launch();
                }
            }
            _ => self.physics(),
        }
        self.prev_plunger = i.plunger;
    }

    fn launch(&mut self) {
        self.balls[0].vel = (0, -(5 + self.charge / 10) * FP);
        self.charge = 0;
        self.ball_pts = 0;
        self.state = State::Playing;
    }

    fn flippers(&mut self, i: Input) {
        for (f, p) in self.flips.iter_mut().zip([i.left, i.right]) {
            let target = if p { f.up } else { f.rest };
            f.dps = (target - f.ang).clamp(-FLIP_SPEED, FLIP_SPEED);
            f.ang += f.dps;
        }
    }

    fn physics(&mut self) {
        let fl = self.flips;
        let mut pts: u32 = 0;
        let mut lit = self.lit;
        let mut inside = self.inside;
        for bi in 0..MAX_BALLS {
            let mut b = self.balls[bi];
            if !b.live {
                continue;
            }
            b.vel.1 += GRAVITY;
            b.vel = (clamp(b.vel.0), clamp(b.vel.1));
            b.vel.0 -= b.vel.0 / 512; // attrito lieve
            b.vel.1 -= b.vel.1 / 512;
            for k in 0..SUB {
                b.pos.0 += b.vel.0 / SUB;
                b.pos.1 += b.vel.1 / SUB;
                for w in WALLS {
                    let (a, c) = px(w);
                    hit_seg(&mut b, a, c, None, E_WALL);
                }
                for f in fl.iter() {
                    // la pala ruota durante il tick: ogni sottopasso usa la sua posa
                    let a = f.ang - f.dps + f.dps * (k + 1) / SUB;
                    hit_seg(&mut b, f.pivot, f.tip_at(a), Some((f.pivot, f.dps)), E_WALL);
                }
                for c in BUMPERS {
                    if hit_circle(&mut b, (c.0 * FP, c.1 * FP), BUMPER_R, E_BUMPER) {
                        pts += 100 * (self.world as u32 + 1);
                    }
                }
                for k in KICKERS {
                    let (a, c) = px(k);
                    let before = b.vel;
                    if let Some(n) = hit_seg(&mut b, a, c, None, E_KICK) {
                        if -dot(before, n) > 2 * FP {
                            b.vel = (b.vel.0 + n.0 * KICK / FP, b.vel.1 + n.1 * KICK / FP);
                            pts += 10;
                        }
                    }
                }
                // ogni impulso può superare VMAX dentro il sottopasso: limita subito
                for (j, s) in SENSORS.iter().enumerate() {
                    let bit = 1u8 << j;
                    let (dx, dy) = (b.pos.0 - s.0 * FP, b.pos.1 - s.1 * FP);
                    if dx * dx + dy * dy >= SENSOR_R * SENSOR_R {
                        inside &= !bit;
                        continue;
                    }
                    if inside & bit != 0 {
                        continue; // già dentro: niente doppio conteggio
                    }
                    inside |= bit;
                    if lit & bit == 0 {
                        lit |= bit;
                        pts += 200;
                    }
                    // la missione avanza su ogni ingresso, anche se il sensore era già acceso
                    if j as u8 == self.mission {
                        self.mission += 1;
                        pts += 500;
                        if self.mission == 3 {
                            // missione compiuta: la porta di marea si apre sul mondo successivo
                            self.mission = 0;
                            self.world = (self.world + 1) % 3;
                            pts += 2000;
                            lit = 0;
                        }
                    }
                }
                b.vel = (clamp(b.vel.0), clamp(b.vel.1));
            }
            if b.pos.1 > (TABLE_H + 3) * FP {
                b.live = false;
            }
            self.balls[bi] = b;
        }
        // palline contro palline: urto elastico a masse uguali
        for i in 0..MAX_BALLS {
            for j in i + 1..MAX_BALLS {
                let (mut a, mut c) = (self.balls[i], self.balls[j]);
                if !a.live || !c.live {
                    continue;
                }
                let d = (a.pos.0 - c.pos.0, a.pos.1 - c.pos.1);
                let d2 = d.0 * d.0 + d.1 * d.1;
                if d2 >= (2 * R) * (2 * R) {
                    continue;
                }
                let n = unit(d, d2);
                let mid = ((a.pos.0 + c.pos.0) / 2, (a.pos.1 + c.pos.1) / 2);
                a.pos = (mid.0 + n.0 * R / FP, mid.1 + n.1 * R / FP);
                c.pos = (mid.0 - n.0 * R / FP, mid.1 - n.1 * R / FP);
                let rel = (a.vel.0 - c.vel.0, a.vel.1 - c.vel.1);
                if dot(rel, n) < 0 {
                    let r = bounce(rel, n, E_BUMPER);
                    let dv = ((r.0 - rel.0) / 2, (r.1 - rel.1) / 2);
                    a.vel = (a.vel.0 + dv.0, a.vel.1 + dv.1);
                    c.vel = (c.vel.0 - dv.0, c.vel.1 - dv.1);
                }
                self.balls[i] = a;
                self.balls[j] = c;
            }
        }
        self.lit = lit;
        self.inside = inside;
        self.score += pts * self.mult;
        self.ball_pts += pts;
        if self.lit == SENSOR_ALL {
            self.lit = 0;
            self.mult = (self.mult + 1).min(5);
            self.score += 1000;
        }
        if !self.multi_done && self.score >= MULTI_AT {
            self.multiball();
        }
        if self.balls.iter().all(|b| !b.live) {
            self.lose_ball();
        }
    }

    /// Due palline extra nei posti liberi, una volta per partita.
    fn multiball(&mut self) {
        self.multi_done = true;
        let mut spawn = [((66 * FP, 150 * FP), (-3 * FP, -6 * FP)), ((74 * FP, 150 * FP), (3 * FP, -6 * FP))].into_iter();
        for b in self.balls.iter_mut().filter(|b| !b.live) {
            match spawn.next() {
                Some((pos, vel)) => *b = Ball { pos, vel, live: true },
                None => break,
            }
        }
    }

    fn lose_ball(&mut self) {
        if self.ball_pts > 0 {
            self.score += 500 * self.mult; // bonus fine pallina, solo se ha fatto punti
        }
        self.lives -= 1;
        if self.lives == 0 {
            self.state = State::Over;
            self.high = self.high.max(self.score);
            return;
        }
        self.balls[0] = Ball { pos: LANE, vel: (0, 0), live: true };
        self.state = State::Ready;
        self.charge = 0;
    }
}

impl Default for Pinball {
    fn default() -> Self {
        Self::new()
    }
}

fn clamp(v: i64) -> i64 {
    v.clamp(-VMAX, VMAX)
}

fn px(w: [i64; 4]) -> (V, V) {
    ((w[0] * FP, w[1] * FP), (w[2] * FP, w[3] * FP))
}

fn isqrt(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// (cos, sin) di `deg`, scala FP.
fn sin_cos(deg: i64) -> V {
    let d = deg.rem_euclid(360);
    let (q, r) = (d / 90, (d % 90) as usize);
    let (s, c) = (SIN[r], SIN[90 - r]);
    match q {
        0 => (c, s),
        1 => (-s, c),
        2 => (-c, -s),
        _ => (s, -c),
    }
}

fn dot(a: V, b: V) -> i64 {
    (a.0 * b.0 + a.1 * b.1) / FP
}

/// Normale unitaria (FP) di `o`, dato `d2 = |o|²` in FP².
fn unit(o: V, d2: i64) -> V {
    if d2 == 0 {
        return (0, -FP);
    }
    let d = isqrt(d2);
    (o.0 * FP / d, o.1 * FP / d)
}

/// Riflessione con restituzione `e` (FP): v' = v - (1+e)(v·n)n.
fn bounce(v: V, n: V, e: i64) -> V {
    let k = (FP + e) * dot(v, n) / FP;
    (v.0 - k * n.0 / FP, v.1 - k * n.1 / FP)
}

/// Pallina contro segmento `a-c`. Con `spin` la superficie si muove (flipper).
/// Ritorna la normale solo se la pallina sta entrando nel segmento.
fn hit_seg(b: &mut Ball, a: V, c: V, spin: Option<(V, i64)>, e: i64) -> Option<V> {
    let (dx, dy) = (c.0 - a.0, c.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = ((((b.pos.0 - a.0) * dx + (b.pos.1 - a.1) * dy) * FP) / len2).clamp(0, FP);
    let q = (a.0 + dx * t / FP, a.1 + dy * t / FP);
    let o = (b.pos.0 - q.0, b.pos.1 - q.1);
    let d2 = o.0 * o.0 + o.1 * o.1;
    if d2 >= R * R {
        return None;
    }
    let n = unit(o, d2);
    b.pos = (q.0 + n.0 * R / FP, q.1 + n.1 * R / FP);
    // velocità tangenziale della superficie: ω × raggio, ω = dps/57 rad/tick
    let surf = match spin {
        Some((p, dps)) => (-(q.1 - p.1) * dps / 57, (q.0 - p.0) * dps / 57),
        None => (0, 0),
    };
    let rel = (b.vel.0 - surf.0, b.vel.1 - surf.1);
    if dot(rel, n) >= 0 {
        return None;
    }
    let r = bounce(rel, n, e);
    b.vel = (r.0 + surf.0, r.1 + surf.1);
    Some(n)
}

/// Pallina contro cerchio (paraurti). `true` solo se sta entrando.
fn hit_circle(b: &mut Ball, c: V, rad: i64, e: i64) -> bool {
    let o = (b.pos.0 - c.0, b.pos.1 - c.1);
    let d2 = o.0 * o.0 + o.1 * o.1;
    let lim = R + rad;
    if d2 >= lim * lim {
        return false;
    }
    let n = unit(o, d2);
    b.pos = (c.0 + n.0 * lim / FP, c.1 + n.1 * lim / FP);
    if dot(b.vel, n) >= 0 {
        return false;
    }
    let hard = -dot(b.vel, n) > 2 * FP; // almeno 2 px/tick di velocità d'urto
    b.vel = bounce(b.vel, n, e);
    hard
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ball(x: i64, y: i64, vx: i64, vy: i64) -> Pinball {
        let mut g = Pinball::new();
        g.state = State::Playing;
        g.balls[0] = Ball { pos: (x * FP, y * FP), vel: (vx * FP, vy * FP), live: true };
        g
    }

    #[test]
    fn isqrt_ok() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(99), 9);
        assert_eq!(isqrt(100), 10);
        assert_eq!(isqrt(1 << 40), 1 << 20);
    }

    #[test]
    fn sin_cos_quadrants() {
        assert_eq!(sin_cos(0), (256, 0));
        assert_eq!(sin_cos(90), (0, 256));
        assert_eq!(sin_cos(-90), (0, -256));
        assert_eq!(sin_cos(180), (-256, 0));
    }

    #[test]
    fn gravity_pulls_down() {
        let mut g = ball(80, 60, 0, 0);
        g.tick(Input::default());
        assert!(g.balls[0].vel.1 > 0);
        assert!(g.balls[0].pos.1 > 60 * FP);
    }

    #[test]
    fn wall_reflects_with_restitution() {
        let mut g = ball(10, 100, -4, 0);
        for _ in 0..10 {
            g.tick(Input::default());
        }
        let b = g.balls[0];
        assert!(b.vel.0 > 0, "la pallina deve rimbalzare dal muro");
        assert!(b.vel.0 < 4 * FP, "non deve guadagnare energia");
        assert!(b.pos.0 >= 4 * FP - 1, "non deve attraversare il muro");
    }

    #[test]
    fn bumper_scores_and_kicks() {
        let mut g = ball(28, 90, 4, 0);
        g.tick(Input::default());
        assert_eq!(g.score, 100);
        assert!(g.balls[0].vel.0 < 0);
    }

    #[test]
    fn no_tunneling_at_max_speed() {
        let mut g = ball(100, 20, 0, -16);
        for _ in 0..3 {
            g.tick(Input::default());
        }
        let b = g.balls[0];
        assert!(b.pos.1 >= FP, "non deve attraversare il soffitto");
        assert!(b.vel.1 > 0, "deve rimbalzare verso il basso");
    }

    #[test]
    fn flipper_rises_when_pressed() {
        let mut g = Pinball::new();
        let left = Input { left: true, ..Input::default() };
        for _ in 0..3 {
            g.tick(left);
        }
        assert_eq!(g.flips[0].ang, -30);
        g.tick(left);
        assert_eq!(g.flips[0].dps, 0);
    }

    #[test]
    fn rollover_scores_once() {
        let mut g = ball(30, 30, 0, 0);
        g.tick(Input::default());
        assert_eq!(g.score, 700, "200 del sensore più 500 della missione");
        assert_eq!(g.lit, 0b001);
        g.tick(Input::default());
        assert_eq!(g.score, 700, "un sensore non paga due volte");
    }

    #[test]
    fn drain_costs_a_life() {
        let mut g = ball(80, 250, 0, 0);
        g.tick(Input::default());
        assert_eq!(g.lives, 2);
        assert_eq!(g.state, State::Ready);
    }

    #[test]
    fn multiball_spawns_extra_balls() {
        let mut g = ball(60, 120, 0, 0);
        g.score = 5000;
        g.tick(Input::default());
        assert_eq!(g.balls.iter().filter(|b| b.live).count(), 3);
    }

    #[test]
    fn plunger_launches_up() {
        let mut g = Pinball::new();
        let hold = Input { plunger: true, ..Input::default() };
        for _ in 0..20 {
            g.tick(hold);
        }
        assert_eq!(g.charge, 40);
        g.tick(Input::default());
        assert_eq!(g.state, State::Playing);
        assert!(g.balls[0].vel.1 < 0);
    }

    #[test]
    fn game_over_keeps_high_score() {
        let mut g = ball(80, 250, 0, 0);
        g.lives = 1;
        g.score = 1234;
        g.tick(Input::default());
        assert_eq!(g.state, State::Over);
        assert_eq!(g.high, g.score);
        let high = g.high;
        g.reset();
        assert_eq!((g.score, g.lives, g.high), (0, 3, high));
    }

    #[test]
    fn sensors_in_order_complete_mission_and_change_world() {
        let mut g = Pinball::new();
        for j in 0..3 {
            let s = SENSORS[j];
            g.balls[0] = Ball { pos: (s.0 * FP, s.1 * FP), vel: (0, 0), live: true };
            g.state = State::Playing;
            g.tick(Input::default());
        }
        assert_eq!(g.world, 1);
        assert_eq!(g.mission, 0);
    }

    #[test]
    fn out_of_order_hits_do_not_block_mission() {
        let mut g = Pinball::new();
        g.state = State::Playing;
        for j in [1usize, 0, 1] {
            let s = SENSORS[j];
            g.balls[0] = Ball { pos: (s.0 * FP, s.1 * FP), vel: (0, 0), live: true };
            g.tick(Input::default());
        }
        assert_eq!(g.mission, 2, "il sensore 1 deve far avanzare la missione dopo il sensore 0");
    }

    #[test]
    fn balls_do_not_overlap() {
        let mut g = ball(70, 150, 0, 0);
        g.balls[1] = Ball { pos: (70 * FP, 150 * FP), vel: (0, 0), live: true };
        g.tick(Input::default());
        let (a, c) = (g.balls[0].pos, g.balls[1].pos);
        let d2 = (a.0 - c.0).pow(2) + (a.1 - c.1).pow(2);
        assert!(d2 >= (2 * R - FP / 4).pow(2), "palline compenetrate: {d2}");
    }

    #[test]
    fn launch_clears_the_lane() {
        let mut g = Pinball::new();
        let hold = Input { plunger: true, ..Input::default() };
        for _ in 0..40 {
            g.tick(hold);
        }
        g.tick(Input::default());
        let mut top = i64::MAX;
        for _ in 0..200 {
            g.tick(Input::default());
            top = top.min(g.balls[0].pos.1);
            assert!(g.balls[0].live);
        }
        assert!(top < 150 * FP, "la pallina non esce dalla corsia");
    }

    #[test]
    fn immediate_drain_pays_no_bonus() {
        let mut g = ball(80, 250, 0, 0);
        g.tick(Input::default());
        assert_eq!(g.score, 0);
    }

    #[test]
    fn resting_ball_on_bumper_does_not_score() {
        let mut g = ball(70, 95, 0, 0);
        for _ in 0..2000 {
            g.tick(Input::default());
        }
        assert!(g.score < 1000, "punti da pallina appoggiata: {}", g.score);
    }

    #[test]
    fn speed_is_capped_after_impulses() {
        let mut g = Pinball::new();
        let mut seed: u64 = 7;
        for _ in 0..20_000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let bits = seed >> 33;
            let inp = Input { left: bits & 1 == 1, right: bits & 2 == 2, plunger: bits & 4 == 4 };
            if g.state == State::Over {
                g.reset();
            }
            g.tick(inp);
            for b in g.balls.iter().filter(|b| b.live) {
                assert!(b.vel.0.abs() <= VMAX && b.vel.1.abs() <= VMAX, "velocità oltre VMAX: {:?}", b.vel);
            }
        }
    }

    #[test]
    fn random_play_stays_in_table() {
        let mut g = Pinball::new();
        let mut seed: u64 = 1;
        for _ in 0..20_000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let bits = seed >> 33;
            let inp = Input {
                left: bits & 1 == 1,
                right: bits & 2 == 2,
                plunger: bits & 4 == 4,
            };
            if g.state == State::Over {
                g.reset();
            }
            g.tick(inp);
            for b in g.balls.iter().filter(|b| b.live) {
                assert!(b.pos.0 >= 0 && b.pos.0 <= TABLE_W * FP, "fuori x: {:?}", b.pos);
            }
        }
    }
}
