//! Gwent-lite: tu contro la CPU, 3 round. Unità, morale, spie e meteo (gelo/sereno).
use crate::draw::{itoa, topbar, Pic, Ui, GOLD, GRN, BLUE, RED, INK, WHITE, YEL};
use crate::{act, arg1, kind, Game, Rng};

const GW: i32 = 120;
const GH: i32 = 170;
const VIOLA: u32 = 0xFF6A1B9A;
const AZZ: u32 = 0xFF4FC3F7;
const GRIGIO: u32 = 0xFF9E9E9E;

/// Carta: k 0 unità, 1 morale, 2 spia, 3 gelo, 4 sereno.
#[derive(Clone, Copy)]
struct G {
    k: u8,
    p: u8,
    row: u8,
    boost: u8,
}

const E: G = G { k: 0, p: 0, row: 0, boost: 0 };

pub struct Gwent {
    rng: Rng,
    hand: [[G; 10]; 2],
    hn: [usize; 2],
    board: [[G; 24]; 2],
    bn: [usize; 2],
    pass: [bool; 2],
    wins: [u8; 2],
    rd: u8,
    frost: bool,
    over: bool,
    msg: &'static str,
}

impl Gwent {
    pub fn new(seed: u32) -> Self {
        let mut s = Self {
            rng: Rng::new(seed),
            hand: [[E; 10]; 2],
            hn: [10, 10],
            board: [[E; 24]; 2],
            bn: [0, 0],
            pass: [false; 2],
            wins: [0; 2],
            rd: 1,
            frost: false,
            over: false,
            msg: "Scegli una carta o passa",
        };
        for side in 0..2 {
            // mazzo da 22: 14 unità, 3 morale, 2 spie, 2 gelo, 1 sereno
            let mut d = [E; 22];
            for (i, c) in d.iter_mut().enumerate() {
                *c = match i {
                    0..=13 => G { k: 0, p: 2 + s.rng.below(9) as u8, row: s.rng.below(3) as u8, boost: 0 },
                    14..=16 => G { k: 1, ..E },
                    17..=18 => G { k: 2, p: 3, ..E },
                    19..=20 => G { k: 3, ..E },
                    _ => G { k: 4, ..E },
                };
            }
            for i in (1..22).rev() {
                let j = s.rng.below(i + 1);
                d.swap(i, j);
            }
            s.hand[side][..10].copy_from_slice(&d[..10]);
        }
        s
    }

    fn done(&self, s: usize) -> bool {
        self.pass[s] || self.hn[s] == 0
    }

    /// Totale effettivo del lato: unità (1 se gelo e mischia) + boost; spie 3.
    fn total(&self, s: usize) -> u32 {
        let mut t = 0;
        for g in &self.board[s][..self.bn[s]] {
            t += match g.k {
                0 => {
                    let base = if self.frost && g.row == 0 { 1 } else { g.p as u32 };
                    base + g.boost as u32
                }
                2 => g.p as u32,
                _ => 0,
            };
        }
        t
    }

    fn has_unit(&self, s: usize) -> bool {
        self.board[s][..self.bn[s]].iter().any(|g| g.k == 0)
    }

    fn put(&mut self, s: usize, g: G) {
        if self.bn[s] < 24 {
            self.board[s][self.bn[s]] = g;
            self.bn[s] += 1;
        }
    }

    /// Gioca la carta `i` della mano del lato `s` e ne applica l'effetto.
    fn play(&mut self, s: usize, i: usize) {
        let g = self.hand[s][i];
        let n = self.hn[s];
        self.hand[s].copy_within(i + 1..n, i);
        self.hn[s] = n - 1;
        match g.k {
            0 => self.put(s, g),
            1 => {
                // morale: +1 a tutte le unità proprie sul board
                for u in self.board[s][..self.bn[s]].iter_mut() {
                    if u.k == 0 {
                        u.boost = u.boost.saturating_add(1);
                    }
                }
            }
            2 => self.put(1 - s, g), // la spia va sul board avversario
            3 => self.frost = true,
            _ => self.frost = false,
        }
    }

    /// Mossa della CPU (una sola).
    fn cpu_move(&mut self) {
        let (c, pl) = (self.total(1), self.total(0));
        let winning = c > pl;
        let losing = c < pl;
        if self.pass[0] && winning {
            self.pass[1] = true;
            return;
        }
        let mut best = 0u32;
        let mut bi = usize::MAX;
        for i in 0..self.hn[1] {
            let g = self.hand[1][i];
            let v = match g.k {
                0 => {
                    if self.frost && g.row == 0 {
                        0
                    } else {
                        g.p as u32
                    }
                }
                1 => {
                    if self.has_unit(1) {
                        3
                    } else {
                        0
                    }
                }
                2 => 0,
                3 => {
                    if !self.frost && losing {
                        4
                    } else {
                        0
                    }
                }
                _ => {
                    if self.frost && losing {
                        4
                    } else {
                        0
                    }
                }
            };
            if v > best {
                best = v;
                bi = i;
            }
        }
        if bi == usize::MAX || (winning && best <= 4) {
            self.pass[1] = true;
        } else {
            self.play(1, bi);
        }
    }

    /// La CPU risponde finché serve; ritorna al giocatore quando può agire.
    fn run_cpu(&mut self) {
        loop {
            if self.done(0) && self.done(1) {
                self.end_round();
                return;
            }
            if self.done(1) {
                return; // il giocatore agisce ancora
            }
            self.cpu_move();
            if !self.done(0) {
                return;
            }
        }
    }

    fn end_round(&mut self) {
        let (a, b) = (self.total(0), self.total(1));
        self.msg = if a > b {
            self.wins[0] += 1;
            "Round vinto da te"
        } else if b > a {
            self.wins[1] += 1;
            "Round vinto dalla CPU"
        } else {
            "Round pari: nessuno vince"
        };
        self.bn = [0, 0];
        self.frost = false;
        self.pass = [false; 2];
        if self.wins[0] == 2 || self.wins[1] == 2 || self.rd == 3 {
            self.over = true;
            self.msg = if self.wins[0] > self.wins[1] {
                "PARTITA VINTA! Premi NUOVA PARTITA"
            } else if self.wins[0] < self.wins[1] {
                "PARTITA PERSA. Premi NUOVA PARTITA"
            } else {
                "PAREGGIO. Premi NUOVA PARTITA"
            };
            return;
        }
        self.rd += 1;
        if self.done(0) {
            self.run_cpu();
        }
    }
}

fn num(p: &mut Pic, x: i32, y: i32, v: u32, h: i32, c: u32) {
    let mut b = [0u8; 12];
    p.text(x, y, itoa(v as i32, &mut b), h, c);
}

/// Disegna una carta di Gwent (GW x GH).
fn gcard(p: &mut Pic, x: i32, y: i32, g: G) {
    let (col, sig): (u32, &str) = match g.k {
        0 => (
            match g.row {
                0 => RED,
                1 => GRN,
                _ => BLUE,
            },
            match g.row {
                0 => "MEL",
                1 => "DIS",
                _ => "ASS",
            },
        ),
        1 => (GOLD, "MOR"),
        2 => (VIOLA, "SPI"),
        3 => (AZZ, "GEL"),
        _ => (GRIGIO, "SER"),
    };
    p.rect(x, y, GW, GH, col);
    p.frame(x, y, GW, GH, 3, INK);
    let mut b = [0u8; 12];
    let big: &str = match g.k {
        0 => itoa(g.p as i32, &mut b),
        1 => "+1",
        2 => "3",
        _ => "",
    };
    p.ctext(x, y + 20, GW, 60, big, 50, WHITE);
    p.ctext(x, y + GH - 50, GW, 30, sig, 24, WHITE);
    if g.k == 0 && g.boost > 0 {
        let t = [b'+', b'0' + g.boost.min(9)];
        p.text(x + 8, y + 8, core::str::from_utf8(&t).unwrap_or(""), 24, WHITE);
    }
}

impl Game for Gwent {
    fn act(&mut self, a: u32) {
        if kind(a) == 0xFF {
            *self = Self::new(self.rng.next());
            return;
        }
        if self.over || self.done(0) {
            return;
        }
        match kind(a) {
            1 => {
                let i = arg1(a) as usize;
                if i < self.hn[0] {
                    self.play(0, i);
                    self.msg = "Scegli una carta o passa";
                    self.run_cpu();
                }
            }
            2 => {
                self.pass[0] = true;
                self.msg = "Hai passato";
                self.run_cpu();
            }
            _ => {}
        }
    }

    fn draw(&self, p: &mut Pic, ui: &mut Ui) {
        ui.reset();
        let mut st = *b"RONDA 0";
        st[6] = b'0' + self.rd;
        let st = core::str::from_utf8(&st).unwrap_or("");
        topbar(p, ui, "GWENT LITE", st);

        // totali e vittorie a sinistra
        p.text(40, 100, "CPU", 30, WHITE);
        num(p, 40, 140, self.total(1), 50, WHITE);
        p.text(40, 340, "TU", 30, WHITE);
        num(p, 40, 380, self.total(0), 50, WHITE);
        if self.frost {
            p.text(40, 460, "GELO", 30, AZZ);
        }
        p.text(40, 560, "CPU", 24, WHITE);
        num(p, 200, 560, self.wins[1] as u32, 24, WHITE);
        p.text(40, 600, "TU", 24, WHITE);
        num(p, 200, 600, self.wins[0] as u32, 24, WHITE);

        // board CPU e tuo (se >10 carte si sovrappongono)
        for (s, y) in [(1usize, 100), (0usize, 340)] {
            let n = self.bn[s];
            let step = if n <= 10 { 140 } else { 1380 / (n as i32 - 1) };
            for i in 0..n {
                gcard(p, 260 + i as i32 * step, y, self.board[s][i]);
            }
        }

        // mano: cliccabile solo se puoi agire
        let can = !self.over;
        for i in 0..self.hn[0] {
            let x = 260 + i as i32 * 140;
            gcard(p, x, 640, self.hand[0][i]);
            if can {
                ui.hit(x, 640, GW, GH, act(1, i as u32, 0));
            }
        }
        ui.button(p, 1650, 340, 220, 64, act(2, 0, 0), "PASSA", can);

        p.text(40, 980, self.msg, 30, YEL);
    }
}
