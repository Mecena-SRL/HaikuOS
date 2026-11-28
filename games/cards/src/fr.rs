//! Giochi con mazzo francese: Solitario, Blackjack, Poker, Otto, Guerra.
use crate::draw::{itoa, topbar, Pic, Ui, CH, CW, GREEN, WHITE, YEL};
use crate::{act, arg1, arg2, deck, kind, shuffle, Card, Game, Pile, Rng};

const SLOT: u32 = 0xFF7FAF8F;
const BACKC: Card = Card { r: 1, s: 0 };

fn red(c: Card) -> bool {
    c.s == 1 || c.s == 2
}

/// Valore per confronti: asso = 14.
fn hv(c: Card) -> usize {
    if c.r == 1 { 14 } else { c.r as usize }
}

/// Numero intero disegnato a testo.
fn num(p: &mut Pic, x: i32, y: i32, n: i32, h: i32, c: u32) {
    let mut b = [0u8; 12];
    p.text(x, y, itoa(n, &mut b), h, c);
}

// ---------------------------------------------------------------- SOLITARIO

#[derive(Clone, Copy, PartialEq)]
enum Src {
    No,
    Waste,
    Tab(u8, u8),
}

pub struct Klondike {
    stock: Pile<52>,
    waste: Pile<52>,
    fnd: [Pile<13>; 4],
    tab: [Pile<52>; 7],
    /// Carte scoperte in fondo a ogni colonna (le altre sono coperte).
    up: [usize; 7],
    src: Src,
    won: bool,
    rng: Rng,
}

impl Klondike {
    pub fn new(seed: u32) -> Self {
        let mut rng = Rng::new(seed);
        let mut d = deck(false);
        shuffle(&mut d, &mut rng);
        let mut g = Self {
            stock: Pile::EMPTY,
            waste: Pile::EMPTY,
            fnd: [Pile::EMPTY; 4],
            tab: [Pile::EMPTY; 7],
            up: [1; 7],
            src: Src::No,
            won: false,
            rng,
        };
        for i in 0..7 {
            for _ in 0..=i {
                if let Some(c) = d.pop() {
                    g.tab[i].push(c);
                }
            }
        }
        while let Some(c) = d.pop() {
            g.stock.push(c);
        }
        g
    }

    /// Dopo aver tolto `removed` carte dalla colonna `s`, scopre la nuova cima.
    fn fix_up(&mut self, s: usize, removed: usize) {
        let n = self.tab[s].n;
        self.up[s] = self.up[s].saturating_sub(removed).max(1).min(n);
    }

    fn src_top(&self) -> Option<Card> {
        match self.src {
            Src::No => None,
            Src::Waste => self.waste.last(),
            Src::Tab(s, i) => {
                let t = &self.tab[s as usize];
                if (i as usize) < t.n { Some(t.c[i as usize]) } else { None }
            }
        }
    }

    fn to_tab(&mut self, d: usize) -> bool {
        let first = match self.src_top() {
            Some(c) => c,
            None => return false,
        };
        let ok = match self.tab[d].last() {
            None => first.r == 13,
            Some(t) => t.r + 1 == first.r && red(t) != red(first),
        };
        match self.src {
            Src::Waste if ok => {
                if let Some(c) = self.waste.pop() {
                    self.tab[d].push(c);
                }
            }
            Src::Tab(s, i) if ok && (s as usize) != d => {
                let (s, i) = (s as usize, i as usize);
                let n = self.tab[s].n;
                for k in i..n {
                    let c = self.tab[s].c[k];
                    self.tab[d].push(c);
                }
                self.tab[s].n = i;
                self.fix_up(s, n - i);
            }
            _ => return false,
        }
        self.src = Src::No;
        true
    }

    fn to_fnd(&mut self, f: usize) -> bool {
        let c = match self.src_top() {
            Some(c) => c,
            None => return false,
        };
        if let Src::Tab(s, i) = self.src {
            // solo la cima di una colonna va in fondazione
            if (i as usize) + 1 != self.tab[s as usize].n {
                return false;
            }
        }
        let ok = match self.fnd[f].last() {
            None => c.r == 1,
            Some(t) => t.s == c.s && t.r + 1 == c.r,
        };
        if !ok {
            return false;
        }
        match self.src {
            Src::Waste => {
                self.waste.pop();
            }
            Src::Tab(s, _) => {
                let s = s as usize;
                self.tab[s].pop();
                self.fix_up(s, 1);
            }
            Src::No => return false,
        }
        self.fnd[f].push(c);
        self.src = Src::No;
        true
    }

    fn tab_click(&mut self, c: usize, b: u8) {
        let n = self.tab[c].n;
        let idx = b as usize;
        let face_up = b != 255 && idx < n && idx >= n - self.up[c];
        match self.src {
            Src::No => {
                if face_up {
                    self.src = Src::Tab(c as u8, b);
                }
            }
            Src::Tab(sc, si) if (sc as usize) == c && si == b => self.src = Src::No,
            _ => {
                if self.to_tab(c) {
                    return;
                }
                self.src = if face_up { Src::Tab(c as u8, b) } else { Src::No };
            }
        }
    }
}

impl Game for Klondike {
    fn act(&mut self, a: u32) {
        match kind(a) {
            0xFF => *self = Self::new(self.rng.next()),
            1 => {
                self.src = Src::No;
                if let Some(c) = self.stock.pop() {
                    self.waste.push(c);
                } else {
                    while let Some(c) = self.waste.pop() {
                        self.stock.push(c);
                    }
                }
            }
            2 => {
                self.src = if self.src == Src::Waste || self.waste.n == 0 { Src::No } else { Src::Waste };
            }
            3 if arg1(a) < 7 => self.tab_click(arg1(a) as usize, arg2(a) as u8),
            4 if arg1(a) < 4 => {
                if self.src != Src::No && !self.to_fnd(arg1(a) as usize) {
                    self.src = Src::No;
                }
            }
            _ => {}
        }
        self.won = self.fnd.iter().all(|f| f.n == 13);
    }

    fn draw(&self, p: &mut Pic, ui: &mut Ui) {
        p.clear(GREEN);
        topbar(p, ui, "SOLITARIO", if self.won { "VITTORIA!" } else { "" });

        if self.stock.n > 0 {
            p.card(60, 110, BACKC, false, false, false);
        } else {
            p.frame(60, 110, CW, CH, 3, SLOT);
        }
        ui.hit(60, 110, CW, CH, act(1, 0, 0));

        match self.waste.last() {
            Some(c) => {
                p.card(230, 110, c, false, true, self.src == Src::Waste);
                ui.hit(230, 110, CW, CH, act(2, 0, 0));
            }
            None => p.frame(230, 110, CW, CH, 3, SLOT),
        }

        for i in 0..4usize {
            let x = 760 + i as i32 * 190;
            match self.fnd[i].last() {
                Some(c) => p.card(x, 110, c, false, true, false),
                None => p.frame(x, 110, CW, CH, 3, SLOT),
            }
            ui.hit(x, 110, CW, CH, act(4, i as u32, 0));
        }

        for c in 0..7usize {
            let x = 60 + c as i32 * 250;
            let y0 = 340;
            let n = self.tab[c].n;
            // colonna intera: destinazione (b = 255)
            ui.hit(x, y0, CW, 700, act(3, c as u32, 255));
            if n == 0 {
                p.frame(x, y0, CW, CH, 3, SLOT);
                continue;
            }
            let nd = n - self.up[c];
            let mut y = y0;
            for j in 0..n {
                let down = j < nd;
                let off = if down { 22 } else { 40 };
                let last = j + 1 == n;
                let sel = matches!(self.src, Src::Tab(sc, si) if (sc as usize) == c && j >= si as usize);
                p.card(x, y, self.tab[c].c[j], false, !down, sel);
                if !down {
                    ui.hit(x, y, CW, if last { CH } else { off }, act(3, c as u32, j as u32));
                }
                y += off;
            }
        }
    }
}

// ---------------------------------------------------------------- BLACKJACK

const BET: i32 = 10;

/// Valore di una mano: asso 11, declassato a 1 se si sballa.
fn bj_val(cs: &[Card]) -> i32 {
    let (mut t, mut a) = (0i32, 0i32);
    for c in cs {
        t += match c.r {
            1 => {
                a += 1;
                11
            }
            11..=13 => 10,
            r => r as i32,
        };
    }
    while t > 21 && a > 0 {
        t -= 10;
        a -= 1;
    }
    t
}

pub struct Blackjack {
    fiches: i32,
    deck: Pile<52>,
    ph: Pile<16>,
    dh: Pile<16>,
    /// 0 pronto, 1 turno del giocatore, 2 mano conclusa.
    phase: u8,
    msg: &'static str,
    rng: Rng,
}

impl Blackjack {
    pub fn new(seed: u32) -> Self {
        let mut rng = Rng::new(seed);
        let mut d = deck(false);
        shuffle(&mut d, &mut rng);
        Self {
            fiches: 100,
            deck: d,
            ph: Pile::EMPTY,
            dh: Pile::EMPTY,
            phase: 0,
            msg: "PREMI GIOCA 10",
            rng,
        }
    }

    fn card(&mut self) -> Card {
        if self.deck.n == 0 {
            self.deck = deck(false);
            shuffle(&mut self.deck, &mut self.rng);
        }
        self.deck.pop().unwrap_or(BACKC)
    }

    fn settle(&mut self, pay: i32, msg: &'static str) {
        self.fiches += pay;
        self.msg = msg;
        self.phase = 2;
    }

    fn start(&mut self) {
        self.fiches -= BET;
        self.ph.clear();
        self.dh.clear();
        for _ in 0..2 {
            let c = self.card();
            self.ph.push(c);
            let d = self.card();
            self.dh.push(d);
        }
        self.phase = 1;
        self.msg = "CARTA O STAI";
        let pb = self.ph.n == 2 && bj_val(self.ph.s()) == 21;
        let db = self.dh.n == 2 && bj_val(self.dh.s()) == 21;
        if pb && db {
            self.settle(BET, "DOPPIO BLACKJACK: PAREGGIO");
        } else if pb {
            self.settle(25, "BLACKJACK! PAGATO 3 A 2");
        } else if db {
            self.settle(0, "IL BANCO HA BLACKJACK");
        }
    }

    fn stand(&mut self) {
        while bj_val(self.dh.s()) < 17 {
            let c = self.card();
            self.dh.push(c);
        }
        let (p, d) = (bj_val(self.ph.s()), bj_val(self.dh.s()));
        if d > 21 || p > d {
            self.settle(20, "HAI VINTO");
        } else if p == d {
            self.settle(BET, "PAREGGIO");
        } else {
            self.settle(0, "HAI PERSO");
        }
    }
}

impl Game for Blackjack {
    fn act(&mut self, a: u32) {
        match kind(a) {
            0xFF => *self = Self::new(self.rng.next()),
            1 if self.phase == 1 => {
                let c = self.card();
                self.ph.push(c);
                if bj_val(self.ph.s()) > 21 {
                    self.settle(0, "SBALLATO! HAI PERSO");
                }
            }
            2 if self.phase == 1 => self.stand(),
            3 if self.phase != 1 && self.fiches >= BET => self.start(),
            _ => {}
        }
    }

    fn draw(&self, p: &mut Pic, ui: &mut Ui) {
        p.clear(GREEN);
        topbar(p, ui, "VENTUNO", "BLACKJACK");
        p.text(60, 80, "FICHES", 26, WHITE);
        num(p, 260, 80, self.fiches, 26, YEL);

        p.text(40, 160, "BANCO", 26, WHITE);
        for (j, c) in self.dh.s().iter().enumerate() {
            let hidden = self.phase == 1 && j == 1;
            p.card(200 + j as i32 * 150, 130, *c, false, !hidden, false);
        }
        if self.phase != 1 && self.dh.n > 0 {
            num(p, 200, 340, bj_val(self.dh.s()), 26, WHITE);
        }

        p.text(40, 620, "TU", 26, WHITE);
        for (j, c) in self.ph.s().iter().enumerate() {
            p.card(200 + j as i32 * 150, 600, *c, false, true, false);
        }
        if self.ph.n > 0 {
            num(p, 200, 810, bj_val(self.ph.s()), 26, WHITE);
        }

        ui.button(p, 1400, 300, 340, 80, act(1, 0, 0), "CARTA", self.phase == 1);
        ui.button(p, 1400, 400, 340, 80, act(2, 0, 0), "STAI", self.phase == 1);
        ui.button(p, 1400, 500, 340, 80, act(3, 0, 0), "GIOCA 10", self.phase != 1 && self.fiches >= BET);

        let msg = if self.phase == 0 && self.fiches < BET { "FICHES ESAURITE" } else { self.msg };
        p.text(60, 920, msg, 36, YEL);
    }
}

// ---------------------------------------------------------------- POKER

const ANTE: i32 = 10;
const CATS: [&str; 9] = [
    "CARTA ALTA",
    "COPPIA",
    "DOPPIA COPPIA",
    "TRIS",
    "SCALA",
    "COLORE",
    "FULL",
    "POKER",
    "SCALA COLORE",
];

/// Punteggio di una mano da 5: categoria in bit alti, poi valori dei gruppi.
fn score(h: &[Card]) -> u32 {
    let mut cnt = [0u32; 15];
    let mut flush = true;
    for c in h {
        cnt[hv(*c)] += 1;
        flush &= c.s == h[0].s;
    }
    // gruppi (conteggio, valore) ordinati per conteggio poi valore
    let mut keys = [0u32; 5];
    let mut ng = 0usize;
    for v in (2..=14usize).rev() {
        if cnt[v] > 0 {
            keys[ng] = (cnt[v] << 4) | v as u32;
            ng += 1;
        }
    }
    for i in 1..ng {
        let mut j = i;
        while j > 0 && keys[j - 1] < keys[j] {
            keys.swap(j - 1, j);
            j -= 1;
        }
    }
    let g0 = keys[0] >> 4;
    let g1 = if ng > 1 { keys[1] >> 4 } else { 0 };
    let hi = keys[0] & 15;
    let mut straight = 0u32;
    if ng == 5 {
        let lo = keys[4] & 15;
        if hi - lo == 4 {
            straight = hi;
        } else if hi == 14 && (keys[1] & 15) == 5 {
            // ruota A-2-3-4-5
            straight = 5;
        }
    }
    let mut packed = 0u32;
    for i in 0..5 {
        let v = if i < ng { keys[i] & 15 } else { 0 };
        packed = (packed << 4) | v;
    }
    let (cat, pk) = if straight > 0 && flush {
        (8, straight << 16)
    } else if g0 == 4 {
        (7, packed)
    } else if g0 == 3 && g1 == 2 {
        (6, packed)
    } else if flush {
        (5, packed)
    } else if straight > 0 {
        (4, straight << 16)
    } else if g0 == 3 {
        (3, packed)
    } else if g0 == 2 && g1 == 2 {
        (2, packed)
    } else if g0 == 2 {
        (1, packed)
    } else {
        (0, packed)
    };
    (cat << 20) | pk
}

pub struct Poker {
    fiches: i32,
    deck: Pile<52>,
    hand: [Card; 5],
    cpu: [Card; 5],
    hold: [bool; 5],
    /// 0 scelta delle carte da tenere, 1 risultato.
    fase: u8,
    msg: &'static str,
    rng: Rng,
}

impl Poker {
    pub fn new(seed: u32) -> Self {
        let mut rng = Rng::new(seed);
        let mut d = deck(false);
        shuffle(&mut d, &mut rng);
        let mut g = Self {
            fiches: 100,
            deck: d,
            hand: [BACKC; 5],
            cpu: [BACKC; 5],
            hold: [false; 5],
            fase: 0,
            msg: "",
            rng,
        };
        g.deal();
        g
    }

    fn take(&mut self) -> Card {
        if self.deck.n == 0 {
            self.deck = deck(false);
            shuffle(&mut self.deck, &mut self.rng);
        }
        self.deck.pop().unwrap_or(BACKC)
    }

    fn deal(&mut self) {
        if self.deck.n < 12 {
            self.deck = deck(false);
            shuffle(&mut self.deck, &mut self.rng);
        }
        self.fiches -= ANTE;
        for i in 0..5 {
            let c = self.take();
            self.hand[i] = c;
            let d = self.take();
            self.cpu[i] = d;
        }
        self.hold = [false; 5];
        self.fase = 0;
        self.msg = "TIENI LE CARTE, POI CAMBIA";
    }

    fn change(&mut self) {
        for i in 0..5 {
            if !self.hold[i] {
                let c = self.take();
                self.hand[i] = c;
            }
        }
        // CPU: tiene coppie e carte >= J
        let mut cnt = [0u8; 15];
        for c in self.cpu.iter() {
            cnt[hv(*c)] += 1;
        }
        let mut keep = [false; 5];
        for i in 0..5 {
            let c = self.cpu[i];
            keep[i] = hv(c) >= 11 || cnt[hv(c)] >= 2;
        }
        for i in 0..5 {
            if !keep[i] {
                let c = self.take();
                self.cpu[i] = c;
            }
        }
        self.fase = 1;
        let (p, b) = (score(&self.hand), score(&self.cpu));
        if p > b {
            self.fiches += 20;
            self.msg = "HAI VINTO LA MANO";
        } else if p == b {
            self.fiches += ANTE;
            self.msg = "PAREGGIO";
        } else {
            self.msg = "IL BANCO VINCE";
        }
    }
}

impl Game for Poker {
    fn act(&mut self, a: u32) {
        match kind(a) {
            0xFF => *self = Self::new(self.rng.next()),
            1 if self.fase == 0 => {
                let i = arg1(a) as usize;
                if i < 5 {
                    self.hold[i] = !self.hold[i];
                }
            }
            2 if self.fase == 0 => self.change(),
            3 if self.fase == 1 && self.fiches >= ANTE => self.deal(),
            _ => {}
        }
    }

    fn draw(&self, p: &mut Pic, ui: &mut Ui) {
        p.clear(GREEN);
        topbar(p, ui, "POKER", "");
        p.text(60, 80, "FICHES", 26, WHITE);
        num(p, 260, 80, self.fiches, 26, YEL);

        p.text(40, 230, "BANCO", 26, WHITE);
        for i in 0..5usize {
            let x = 300 + i as i32 * 170;
            p.card(x, 150, self.cpu[i], false, self.fase == 1, false);
        }
        if self.fase == 1 {
            p.text(300, 360, CATS[(score(&self.cpu) >> 20) as usize], 28, WHITE);
        }

        for i in 0..5usize {
            let x = 300 + i as i32 * 170;
            let held = self.hold[i] && self.fase == 0;
            p.card(x, 600, self.hand[i], false, true, held);
            if self.fase == 0 {
                ui.hit(x, 600, CW, CH, act(1, i as u32, 0));
                if held {
                    p.text(x + 30, 600 + CH + 12, "TIENI", 28, YEL);
                }
            }
        }
        if self.fase == 1 {
            p.text(300, 840, CATS[(score(&self.hand) >> 20) as usize], 28, WHITE);
        }

        ui.button(p, 1400, 500, 340, 80, act(2, 0, 0), "CAMBIA", self.fase == 0);
        ui.button(p, 1400, 600, 340, 80, act(3, 0, 0), "NUOVA MANO", self.fase == 1 && self.fiches >= ANTE);

        let msg = if self.fase == 1 && self.fiches < ANTE { "FICHES ESAURITE" } else { self.msg };
        p.text(60, 930, msg, 36, YEL);
    }
}

// ---------------------------------------------------------------- OTTO

const SUITS: [&str; 4] = ["PICCHE", "CUORI", "QUADRI", "FIORI"];

fn spacing(n: usize) -> i32 {
    (1250 / (n.max(1) as i32)).min(140)
}

/// Seme più frequente in una mano.
fn best_suit(h: &Pile<52>) -> u8 {
    let mut n = [0u32; 4];
    for c in h.s() {
        n[(c.s & 3) as usize] += 1;
    }
    let mut b = 0usize;
    for s in 1..4 {
        if n[s] > n[b] {
            b = s;
        }
    }
    b as u8
}

pub struct Otto {
    stock: Pile<52>,
    disc: Pile<52>,
    pl: Pile<52>,
    cp: Pile<52>,
    /// Seme corrente (cambia dopo un 8).
    cur: u8,
    /// Il giocatore ha giocato un 8 e deve scegliere il seme.
    wait: bool,
    /// 0 giocatore, 1 banco.
    turn: u8,
    /// 0 in corso, 1 vince il giocatore, 2 vince il banco.
    over: u8,
    rng: Rng,
}

impl Otto {
    pub fn new(seed: u32) -> Self {
        let mut rng = Rng::new(seed);
        let mut d = deck(false);
        shuffle(&mut d, &mut rng);
        let mut g = Self {
            stock: Pile::EMPTY,
            disc: Pile::EMPTY,
            pl: Pile::EMPTY,
            cp: Pile::EMPTY,
            cur: 0,
            wait: false,
            turn: 0,
            over: 0,
            rng,
        };
        for _ in 0..7 {
            if let Some(c) = d.pop() {
                g.pl.push(c);
            }
            if let Some(c) = d.pop() {
                g.cp.push(c);
            }
        }
        while let Some(c) = d.pop() {
            g.stock.push(c);
        }
        if let Some(c) = g.stock.pop() {
            g.disc.push(c);
            g.cur = c.s;
        }
        g
    }

    fn ok(&self, c: Card) -> bool {
        c.r == 8 || c.s == self.cur || self.disc.last().map_or(false, |t| t.r == c.r)
    }

    fn any_ok(&self) -> bool {
        self.pl.s().iter().any(|c| self.ok(*c))
    }

    /// Pesca dallo stock; se vuoto ricicla lo scarto (tranne la cima).
    fn draw_one(&mut self) -> Option<Card> {
        if self.stock.n == 0 && self.disc.n > 1 {
            let top = self.disc.pop();
            while let Some(c) = self.disc.pop() {
                self.stock.push(c);
            }
            if let Some(t) = top {
                self.disc.push(t);
            }
            shuffle(&mut self.stock, &mut self.rng);
        }
        self.stock.pop()
    }

    /// Turno del banco: gioca la prima non-8 giocabile, altrimenti un 8, altrimenti pesca.
    fn cpu(&mut self) {
        loop {
            let mut hit = None;
            for i in 0..self.cp.n {
                let c = self.cp.c[i];
                if c.r != 8 && self.ok(c) {
                    hit = Some(i);
                    break;
                }
            }
            if let Some(i) = hit {
                let c = self.cp.remove(i);
                self.disc.push(c);
                self.cur = c.s;
                self.turn = 0;
                if self.cp.n == 0 {
                    self.over = 2;
                }
                return;
            }
            let mut eight = None;
            for i in 0..self.cp.n {
                if self.cp.c[i].r == 8 {
                    eight = Some(i);
                    break;
                }
            }
            if let Some(i) = eight {
                let c = self.cp.remove(i);
                self.disc.push(c);
                self.turn = 0;
                if self.cp.n == 0 {
                    self.over = 2;
                    return;
                }
                self.cur = best_suit(&self.cp);
                return;
            }
            match self.draw_one() {
                Some(c) => self.cp.push(c),
                None => {
                    self.turn = 0;
                    return;
                }
            }
        }
    }

    fn msg(&self) -> &'static str {
        if self.over == 1 {
            "HAI VINTO!"
        } else if self.over == 2 {
            "HAI PERSO"
        } else if self.wait {
            "SCEGLI IL SEME"
        } else if self.turn == 0 {
            "TOCCA A TE"
        } else {
            "TOCCA AL BANCO"
        }
    }
}

impl Game for Otto {
    fn act(&mut self, a: u32) {
        let play = self.turn == 0 && !self.wait && self.over == 0;
        match kind(a) {
            0xFF => *self = Self::new(self.rng.next()),
            1 if play => {
                let i = arg1(a) as usize;
                if i < self.pl.n && self.ok(self.pl.c[i]) {
                    let c = self.pl.remove(i);
                    self.disc.push(c);
                    self.cur = c.s;
                    if self.pl.n == 0 {
                        self.over = 1;
                    } else if c.r == 8 {
                        self.wait = true;
                    } else {
                        self.turn = 1;
                        self.cpu();
                    }
                }
            }
            2 if play => {
                if !self.any_ok() {
                    let mut found = false;
                    while let Some(c) = self.draw_one() {
                        self.pl.push(c);
                        if self.ok(c) {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        self.turn = 1;
                        self.cpu();
                    }
                }
            }
            3 if self.wait && self.over == 0 => {
                self.cur = (arg1(a) & 3) as u8;
                self.wait = false;
                self.turn = 1;
                self.cpu();
            }
            _ => {}
        }
    }

    fn draw(&self, p: &mut Pic, ui: &mut Ui) {
        p.clear(GREEN);
        topbar(p, ui, "CRAZY EIGHTS", "");
        let dummy = BACKC;
        let sp_cp = spacing(self.cp.n);
        for i in 0..self.cp.n {
            p.card(60 + i as i32 * sp_cp, 100, dummy, false, false, false);
        }
        p.text(60, 300, "BANCO", 26, WHITE);

        if self.stock.n > 0 {
            p.card(600, 380, dummy, false, false, false);
        } else {
            p.frame(600, 380, CW, CH, 3, SLOT);
        }
        if let Some(c) = self.disc.last() {
            p.card(800, 380, c, false, true, false);
        }
        p.text(1000, 370, "SEME", 26, WHITE);
        p.suit(1080, 440, 40, false, self.cur);

        let play = self.turn == 0 && !self.wait && self.over == 0;
        let sp = spacing(self.pl.n);
        p.text(60, 680, "TU", 26, WHITE);
        for i in 0..self.pl.n {
            let c = self.pl.c[i];
            let x = 60 + i as i32 * sp;
            let w = if i + 1 == self.pl.n { CW } else { sp };
            p.card(x, 720, c, false, true, play && self.ok(c));
            if play {
                ui.hit(x, 720, w, CH, act(1, i as u32, 0));
            }
        }

        ui.button(p, 1400, 560, 340, 80, act(2, 0, 0), "PESCA", play);
        if self.wait {
            for s in 0..4usize {
                ui.button(p, 1400, 200 + s as i32 * 90, 340, 80, act(3, s as u32, 0), SUITS[s], true);
            }
        }
        p.text(60, 960, self.msg(), 36, YEL);
    }
}

// ---------------------------------------------------------------- GUERRA

pub struct War {
    a: Pile<52>,
    b: Pile<52>,
    /// Piatto: carte giocate nel giro corrente o in una guerra.
    pot: Pile<52>,
    la: Option<Card>,
    lb: Option<Card>,
    /// 0 in corso, 1 vince il giocatore, 2 perde.
    over: u8,
    msg: &'static str,
    rng: Rng,
}

impl War {
    pub fn new(seed: u32) -> Self {
        let mut rng = Rng::new(seed);
        let mut d = deck(false);
        shuffle(&mut d, &mut rng);
        let mut g = Self {
            a: Pile::EMPTY,
            b: Pile::EMPTY,
            pot: Pile::EMPTY,
            la: None,
            lb: None,
            over: 0,
            msg: "GIRA LA CARTA",
            rng,
        };
        let mut t = false;
        while let Some(c) = d.pop() {
            if t {
                g.b.push(c);
            } else {
                g.a.push(c);
            }
            t = !t;
        }
        g
    }

    fn gira(&mut self) {
        if self.over != 0 {
            return;
        }
        let (x, y) = match (self.a.front(), self.b.front()) {
            (Some(x), Some(y)) => (x, y),
            _ => {
                self.over = if self.a.n == 0 { 2 } else { 1 };
                return;
            }
        };
        self.pot.push(x);
        self.pot.push(y);
        self.la = Some(x);
        self.lb = Some(y);
        let (rx, ry) = (hv(x), hv(y));
        if rx == ry {
            self.msg = "GUERRA!";
        } else {
            let a_wins = rx > ry;
            while let Some(c) = self.pot.front() {
                if a_wins {
                    self.a.push(c);
                } else {
                    self.b.push(c);
                }
            }
            self.msg = if a_wins { "PRENDI TU IL PIATTO" } else { "IL BANCO PRENDE IL PIATTO" };
        }
        if self.a.n == 0 {
            self.over = 2;
            self.msg = "HAI PERSO";
        } else if self.b.n == 0 {
            self.over = 1;
            self.msg = "HAI VINTO!";
        }
    }
}

impl Game for War {
    fn act(&mut self, a: u32) {
        match kind(a) {
            0xFF => *self = Self::new(self.rng.next()),
            1 => self.gira(),
            _ => {}
        }
    }

    fn draw(&self, p: &mut Pic, ui: &mut Ui) {
        p.clear(GREEN);
        topbar(p, ui, "GUERRA", "");
        let dummy = BACKC;

        p.text(200, 130, "TU", 30, WHITE);
        if self.a.n > 0 {
            p.card(200, 180, dummy, false, false, false);
        } else {
            p.frame(200, 180, CW, CH, 3, SLOT);
        }
        num(p, 200, 390, self.a.n as i32, 30, WHITE);

        p.text(1500, 130, "BANCO", 30, WHITE);
        if self.b.n > 0 {
            p.card(1500, 180, dummy, false, false, false);
        } else {
            p.frame(1500, 180, CW, CH, 3, SLOT);
        }
        num(p, 1500, 390, self.b.n as i32, 30, WHITE);

        if let Some(c) = self.la {
            p.card(560, 180, c, false, true, false);
        }
        if let Some(c) = self.lb {
            p.card(1140, 180, c, false, true, false);
        }

        p.text(700, 480, "PIATTO", 26, WHITE);
        num(p, 900, 480, self.pot.n as i32, 26, WHITE);

        ui.button(p, 760, 700, 400, 100, act(1, 0, 0), "GIRA", self.over == 0);
        p.text(60, 920, self.msg, 36, YEL);
    }
}
