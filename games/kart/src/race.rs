//! Gara: giri, posizioni, oggetti, collisioni, coppa. Nessuna grafica.

use crate::{
    ai,
    fx::{dir, div, int, isqrt, mul, Fx, ONE},
    kart::{Input, Item, Kart, SPIN_TICKS},
    track::Track,
};

pub const MAX_RACERS: usize = 8;
pub const N_BOXES: usize = 8;
const N_HAZ: usize = 16;
/// Punti per posizione, dal 1° all'8°.
pub const POINTS: [u32; MAX_RACERS] = [15, 12, 10, 9, 8, 7, 6, 5];

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Haz {
    #[default]
    None,
    Banana,
    Shell,
}

/// Oggetto a terra o in volo. `arm`: tick prima che possa colpire (evita di colpire chi l'ha lanciato).
#[derive(Clone, Copy, Default)]
pub struct Hazard {
    pub kind: Haz,
    pub x: Fx,
    pub y: Fx,
    pub h: Fx,
    pub life: u16,
    pub arm: u16,
    pub owner: u8,
}

fn d2(dx: Fx, dy: Fx) -> i64 {
    let (a, b) = (dx as i64, dy as i64);
    a * a + b * b
}

fn within(dx: Fx, dy: Fx, r: i32) -> bool {
    d2(dx, dy) < ((r as i64) << 16).pow(2)
}

/// Giri completati e traguardo. Un passaggio avanti dall'ultimo quarto al primo conta un giro.
fn lap(k: &mut Kart, seg: usize, n: usize, laps: u8, tick: u32) {
    let (q, f) = (n * 3 / 4, n / 4);
    if k.seg >= q && seg < f {
        k.count = k.count.saturating_add(1);
        if k.count >= laps && k.finish.is_none() {
            k.finish = Some(tick);
        }
    } else if k.seg < f && seg >= q {
        k.count = k.count.saturating_sub(1);
    }
    k.seg = seg;
}

/// Direzione di partenza: l'angolo che punta più avanti lungo il segmento `seg`.
fn heading_of(track: &Track, seg: usize) -> Fx {
    let (ax, ay) = track.point(seg, 0);
    let (bx, by) = track.point(seg, ONE);
    let (sx, sy) = (bx - ax, by - ay);
    let (mut best, mut ang) = (i32::MIN, 0);
    for a in 0..256 {
        let (dx, dy) = dir(int(a));
        let d = mul(dx, sx) + mul(dy, sy);
        if d > best {
            best = d;
            ang = a;
        }
    }
    int(ang)
}

pub struct Race<'a> {
    pub track: &'a Track,
    pub karts: [Kart; MAX_RACERS],
    pub n: usize,
    /// Kart in ordine di posizione (0 = primo).
    pub order: [u8; MAX_RACERS],
    /// Tick alla riattivazione per ogni scatola (0 = attiva).
    pub boxes: [u16; N_BOXES],
    pub haz: [Hazard; N_HAZ],
    pub tick: u32,
    pub countdown: u16,
    rng: u32,
}

impl<'a> Race<'a> {
    /// Gara con `n` kart. Il kart 0 è il giocatore (se passato l'input), gli altri sono IA.
    pub fn new(track: &'a Track, n: usize, seed: u32) -> Self {
        let n = n.clamp(1, MAX_RACERS);
        let h0 = heading_of(track, 0);
        let mut karts = [Kart::default(); MAX_RACERS];
        for i in 0..n {
            let t = ONE * (n - i) as Fx / (n as Fx + 2);
            let (x, y) = track.point(0, t);
            karts[i] = Kart { x, y, h: h0, t, ..Default::default() };
        }
        let mut r = Race {
            track,
            karts,
            n,
            order: [0; MAX_RACERS],
            boxes: [0; N_BOXES],
            haz: [Hazard::default(); N_HAZ],
            tick: 0,
            countdown: 120,
            rng: seed | 1,
        };
        r.sort_order();
        r
    }

    pub fn finished(&self) -> bool {
        self.karts[..self.n].iter().all(|k| k.finish.is_some())
    }

    /// Avanza di un tick. `human` = comandi del giocatore (kart 0); `None` = guida l'IA.
    pub fn tick(&mut self, human: Option<Input>) {
        let track = self.track;
        let n = track.n();
        let wall = track.wall();
        let counting = self.countdown > 0;
        self.countdown = self.countdown.saturating_sub(1);

        for i in 0..self.n {
            let k = self.karts[i];
            let rank = self.rank(i);
            let near = track.nearest(k.x, k.y);
            let mut inp = match (i, human) {
                (0, Some(h)) => h,
                _ => ai::drive(track, &k, &near),
            };
            if i > 0 {
                inp.item = self.rand(60) == 0;
            }
            if counting || k.finish.is_some() {
                inp = Input::default();
            }
            if inp.item && k.item != Item::None {
                self.use_item(i);
            }
            let off = near.d2 > track.off_d2();
            let kk = &mut self.karts[i];
            kk.drive(inp, off);
            // Il kart può uscire di pista, ma non oltre il muro: lo riporta sul bordo.
            let post = track.nearest(kk.x, kk.y);
            if post.d2 > (wall as i64).pow(2) {
                let r = div(wall, isqrt(post.d2 as u64) as Fx);
                kk.x = post.x + mul(kk.x - post.x, r);
                kk.y = post.y + mul(kk.y - post.y, r);
                kk.v /= 2;
            }
            lap(kk, post.seg, n, track.laps, self.tick);
            kk.t = post.t;

            for b in 0..N_BOXES {
                if self.boxes[b] != 0 {
                    continue;
                }
                let (bx, by) = track.point(b * n / N_BOXES, ONE / 2);
                let (kx, ky) = (self.karts[i].x, self.karts[i].y);
                if within(kx - bx, ky - by, 14) {
                    if self.karts[i].item == Item::None {
                        self.karts[i].item = self.roll(rank);
                    }
                    self.boxes[b] = 240;
                }
            }
        }

        // Collisioni tra kart: si spingono via e perdono un po' di velocità.
        for a in 0..self.n {
            for b in a + 1..self.n {
                let (dx, dy) = (self.karts[a].x - self.karts[b].x, self.karts[a].y - self.karts[b].y);
                if d2(dx, dy) > 0 && within(dx, dy, 12) {
                    let d = isqrt(d2(dx, dy) as u64) as Fx;
                    let push = (int(12) - d) / 2;
                    let (nx, ny) = (div(dx, d), div(dy, d));
                    self.karts[a].x += mul(nx, push);
                    self.karts[a].y += mul(ny, push);
                    self.karts[b].x -= mul(nx, push);
                    self.karts[b].y -= mul(ny, push);
                    self.karts[a].v = mul(self.karts[a].v, ONE * 7 / 8);
                    self.karts[b].v = mul(self.karts[b].v, ONE * 7 / 8);
                }
            }
        }

        for h in 0..N_HAZ {
            let mut hz = self.haz[h];
            if hz.kind == Haz::None {
                continue;
            }
            hz.life = hz.life.saturating_sub(1);
            hz.arm = hz.arm.saturating_sub(1);
            if hz.kind == Haz::Shell {
                let (dx, dy) = dir(hz.h);
                hz.x += mul(dx, int(5));
                hz.y += mul(dy, int(5));
            }
            if hz.life == 0 {
                hz.kind = Haz::None;
            } else if hz.arm == 0 {
                for j in 0..self.n {
                    let kj = &mut self.karts[j];
                    if kj.star > 0 || kj.spin > 0 {
                        continue;
                    }
                    if within(kj.x - hz.x, kj.y - hz.y, 8) {
                        kj.spin = SPIN_TICKS;
                        kj.v = 0;
                        hz.kind = Haz::None;
                        break;
                    }
                }
            }
            self.haz[h] = hz;
        }

        for b in self.boxes.iter_mut() {
            *b = b.saturating_sub(1);
        }
        self.sort_order();
        self.tick += 1;
    }

    fn use_item(&mut self, i: usize) {
        let k = self.karts[i];
        self.karts[i].item = Item::None;
        let (dx, dy) = dir(k.h);
        let hz = match k.item {
            Item::Mushroom => {
                self.karts[i].boost = 40;
                return;
            }
            Item::Star => {
                self.karts[i].star = 240;
                self.karts[i].boost = 240;
                return;
            }
            Item::Banana => Hazard {
                kind: Haz::Banana,
                x: k.x - mul(dx, int(10)),
                y: k.y - mul(dy, int(10)),
                h: 0,
                life: 900,
                arm: 20,
                owner: i as u8,
            },
            Item::Shell => Hazard {
                kind: Haz::Shell,
                x: k.x + mul(dx, int(12)),
                y: k.y + mul(dy, int(12)),
                h: k.h,
                life: 200,
                arm: 10,
                owner: i as u8,
            },
            Item::None => return,
        };
        if let Some(s) = self.haz.iter_mut().find(|s| s.kind == Haz::None) {
            *s = hz;
        }
    }

    /// Oggetto casuale: chi è davanti prende oggetti più deboli.
    fn roll(&mut self, rank: usize) -> Item {
        let r = self.rand(100);
        if rank < 3 {
            if r < 50 { Item::Mushroom } else { Item::Banana }
        } else if rank < 6 {
            if r < 30 {
                Item::Mushroom
            } else if r < 70 {
                Item::Shell
            } else {
                Item::Banana
            }
        } else if r < 50 {
            Item::Shell
        } else if r < 75 {
            Item::Star
        } else {
            Item::Mushroom
        }
    }

    fn rand(&mut self, m: u32) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng % m
    }

    fn rank(&self, i: usize) -> usize {
        self.order[..self.n].iter().position(|&o| o as usize == i).unwrap_or(0)
    }

    /// Chiave di ordinamento: più alta = più avanti. I finiti stanno sopra i non finiti.
    fn key(&self, i: usize) -> i64 {
        let k = &self.karts[i];
        match k.finish {
            Some(f) => (1i64 << 50) - f as i64,
            None => {
                let span = n_span(self.track.n());
                k.count as i64 * span + k.seg as i64 * ONE as i64 + k.t as i64
            }
        }
    }

    pub fn sort_order(&mut self) {
        let mut keys = [0i64; MAX_RACERS];
        for i in 0..self.n {
            keys[i] = self.key(i);
            self.order[i] = i as u8;
        }
        for a in 1..self.n {
            let mut b = a;
            while b > 0 && keys[self.order[b - 1] as usize] < keys[self.order[b] as usize] {
                self.order.swap(b - 1, b);
                b -= 1;
            }
        }
    }
}

fn n_span(n: usize) -> i64 {
    n as i64 * ONE as i64
}

/// Gara intera solo IA. Ritorna l'ordine finale e i tick impiegati.
pub fn run_ai(track: &Track, n: usize, seed: u32) -> ([u8; MAX_RACERS], u32) {
    let mut r = Race::new(track, n, seed);
    while !r.finished() && r.tick < 60_000 {
        r.tick(None);
    }
    (r.order, r.tick)
}

/// Coppa: una gara per circuito, punti per posizione. Ritorna i punti per kart.
pub fn run_cup(tracks: &[Track], n: usize) -> [u32; MAX_RACERS] {
    let mut pts = [0u32; MAX_RACERS];
    for (ti, t) in tracks.iter().enumerate() {
        let (order, _) = run_ai(t, n, 1 + ti as u32 * 7919);
        for r in 0..n {
            pts[order[r] as usize] += POINTS[r];
        }
    }
    pts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::track::TRACKS;

    fn kart_at(seg: usize, count: u8) -> Kart {
        Kart { seg, count, ..Default::default() }
    }

    #[test]
    fn forward_crossing_counts_a_lap() {
        let mut k = kart_at(15, 0);
        lap(&mut k, 0, 16, 3, 10);
        assert_eq!(k.count, 1);
        assert!(k.finish.is_none());
    }

    #[test]
    fn no_lap_without_crossing_line() {
        let mut k = kart_at(5, 0);
        lap(&mut k, 6, 16, 3, 10);
        assert_eq!(k.count, 0);
    }

    #[test]
    fn backward_crossing_removes_a_lap() {
        let mut k = kart_at(0, 1);
        lap(&mut k, 15, 16, 3, 10);
        assert_eq!(k.count, 0);
    }

    #[test]
    fn finishes_after_last_lap() {
        let mut k = kart_at(0, 0);
        for lap_no in 0..3u32 {
            for s in 1..16 {
                lap(&mut k, s, 16, 3, lap_no);
            }
            lap(&mut k, 0, 16, 3, 100 + lap_no);
        }
        assert_eq!(k.count, 3);
        assert_eq!(k.finish, Some(102));
    }

    #[test]
    fn leader_is_first() {
        let mut r = Race::new(&TRACKS[0], 3, 1);
        r.karts[2].count = 1;
        r.sort_order();
        assert_eq!(r.order[0], 2);
    }

    #[test]
    fn finished_ranks_first() {
        let mut r = Race::new(&TRACKS[0], 3, 1);
        r.karts[1].finish = Some(500);
        r.karts[0].count = 2;
        r.sort_order();
        assert_eq!(r.order[0], 1);
    }

    #[test]
    fn ai_race_completes() {
        let mut r = Race::new(&TRACKS[0], 8, 1);
        while !r.finished() && r.tick < 60_000 {
            r.tick(None);
        }
        assert!(r.finished(), "gara non finita in {} tick", r.tick);
    }

    #[test]
    fn cup_awards_every_position() {
        let pts = run_cup(&TRACKS[..2], 8);
        let total: u32 = pts.iter().sum();
        assert_eq!(total, 2 * POINTS.iter().sum::<u32>());
    }
}
