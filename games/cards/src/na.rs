//! Giochi napoletani: Scopa, Briscola/Tressette (`Trick`), Sette e mezzo.
//! Mazzo a 40 carte (`deck(true)`). Senza allocazione.

use crate::draw::{itoa, sum_text, text_w, topbar, Pic, Ui, CH, CW, GREEN, WHITE, YEL};
use crate::{act, arg1, deck, kind, shuffle, Card, Game, Pile, Rng};

/// Massimo di carte del tavolo considerate per le catture (limita l'enumerazione).
const LIM: usize = 20;

fn num(p: &mut Pic, x: i32, y: i32, n: i32, h: i32, c: u32) {
    let mut b = [0u8; 12];
    p.text(x, y, itoa(n, &mut b), h, c);
}

/// Etichetta seguita da un numero; ritorna la x della voce successiva.
fn kv(p: &mut Pic, x: i32, y: i32, k: &str, n: i32) -> i32 {
    let h = 26;
    p.text(x, y, k, h, WHITE);
    let x2 = x + text_w(k, h) + 12;
    let mut b = [0u8; 12];
    let s = itoa(n, &mut b);
    p.text(x2, y, s, h, YEL);
    x2 + text_w(s, h) + 50
}

// ---------------------------------------------------------------- SCOPA

/// Sottoinsieme di `v` con somma `need` e il maggior numero di carte: (carte, maschera).
fn best(v: &[u32], need: u32, i: usize, mask: u64, cnt: u32, out: &mut (u32, u64)) {
    if need == 0 {
        if cnt > out.0 {
            *out = (cnt, mask);
        }
        return;
    }
    if i >= v.len() {
        return;
    }
    if v[i] <= need {
        best(v, need - v[i], i + 1, mask | (1u64 << i), cnt + 1, out);
    }
    best(v, need, i + 1, mask, cnt, out);
}

fn prim(r: u8) -> u32 {
    match r {
        7 => 21,
        6 => 18,
        1 => 16,
        5 => 15,
        4 => 14,
        3 => 13,
        2 => 12,
        _ => 10,
    }
}

fn denari(p: &Pile<40>) -> u32 {
    p.s().iter().filter(|c| c.s == 1).count() as u32
}

fn ha_sette(p: &Pile<40>) -> bool {
    p.s().iter().any(|c| c.r == 7 && c.s == 1)
}

/// Primiera: per seme il valore migliore, somma dei quattro semi.
fn primiera(p: &Pile<40>) -> u32 {
    let mut b = [0u32; 4];
    for c in p.s() {
        let s = (c.s & 3) as usize;
        b[s] = b[s].max(prim(c.r));
    }
    b[0] + b[1] + b[2] + b[3]
}

/// Assegna un punto al giocatore con il valore maggiore (nessuno in parità).
fn bonus(a: u32, b: u32, rb: &mut [u32; 2]) {
    if a > b {
        rb[0] += 1;
    } else if b > a {
        rb[1] += 1;
    }
}

pub struct Scopa {
    rng: Rng,
    mazzo: Pile<52>,
    tav: Pile<40>,
    mano: [Pile<3>; 2], // 0 = tu, 1 = CPU
    prese: [Pile<40>; 2],
    pts: [u32; 2],   // punti partita
    scope: [u32; 2], // scope nel round
    rb: [u32; 2],    // punti del round appena chiuso
    primo: usize,    // chi apre il round
    turno: usize,
    ultimo: Option<usize>, // ultimo a prendere
    gi: usize,       // carta in mano in attesa di presa
    mask: u64,       // carte del tavolo selezionate
    fase: u8,        // 0 gioco, 1 scelta prese, 2 round finito, 3 partita finita
}

impl Scopa {
    pub fn new(seed: u32) -> Self {
        let mut s = Scopa {
            rng: Rng::new(seed),
            mazzo: deck(true),
            tav: Pile::EMPTY,
            mano: [Pile::EMPTY; 2],
            prese: [Pile::EMPTY; 2],
            pts: [0; 2],
            scope: [0; 2],
            rb: [0; 2],
            primo: 0,
            turno: 0,
            ultimo: None,
            gi: 0,
            mask: 0,
            fase: 0,
        };
        s.apri_round();
        s
    }

    /// Mescola, mette 4 carte sul tavolo, distribuisce e avvia il round.
    fn apri_round(&mut self) {
        self.mazzo = deck(true);
        shuffle(&mut self.mazzo, &mut self.rng);
        self.tav.clear();
        self.prese = [Pile::EMPTY; 2];
        self.scope = [0; 2];
        self.ultimo = None;
        self.mask = 0;
        self.fase = 0;
        self.turno = self.primo;
        for _ in 0..4 {
            if let Some(c) = self.mazzo.pop() {
                self.tav.push(c);
            }
        }
        self.distribuisci();
        self.tocca_cpu();
    }

    fn distribuisci(&mut self) {
        for _ in 0..3 {
            for p in 0..2 {
                if let Some(c) = self.mazzo.pop() {
                    self.mano[p].push(c);
                }
            }
        }
    }

    fn lim(&self) -> usize {
        self.tav.n.min(LIM)
    }

    /// Sottoinsiemi del tavolo che sommano al valore `val`: (carte, maschera).
    fn cattura(&self, val: u32) -> (u32, u64) {
        let lim = self.lim();
        let mut v = [0u32; LIM];
        for j in 0..lim {
            v[j] = self.tav.c[j].r as u32;
        }
        let mut out = (0u32, 0u64);
        best(&v[..lim], val, 0, 0, 0, &mut out);
        out
    }

    fn somma_mask(&self) -> u32 {
        let mut s = 0;
        for j in 0..self.lim() {
            if (self.mask >> j) & 1 == 1 {
                s += self.tav.c[j].r as u32;
            }
        }
        s
    }

    /// Ultima giocata del round (mazzo e mani vuoti): la scopa non conta.
    fn ultima(&self) -> bool {
        self.mazzo.n == 0 && self.mano[0].n == 0 && self.mano[1].n == 0
    }

    /// Porta in presa la carta `c` e le carte di `mask` tra le `lim` del tavolo.
    fn prende(&mut self, p: usize, c: Card, mask: u64, lim: usize) {
        for j in (0..lim).rev() {
            if (mask >> j) & 1 == 1 {
                let t = self.tav.remove(j);
                self.prese[p].push(t);
            }
        }
        self.prese[p].push(c);
        self.ultimo = Some(p);
        if self.tav.n == 0 && !self.ultima() {
            self.scope[p] += 1;
            self.pts[p] += 1;
        }
    }

    fn gioca(&mut self, i: usize) {
        if i >= self.mano[0].n {
            return;
        }
        let v = self.mano[0].c[i].r as u32;
        let (n, _) = self.cattura(v);
        if n == 0 {
            let c = self.mano[0].remove(i);
            self.tav.push(c);
            self.passa();
            self.tocca_cpu();
        } else {
            self.gi = i;
            self.mask = 0;
            self.fase = 1;
        }
    }

    fn prendi(&mut self) {
        let i = self.gi;
        if self.mask == 0 || self.somma_mask() != self.mano[0].c[i].r as u32 {
            return;
        }
        let c = self.mano[0].remove(i);
        let (lim, m) = (self.lim(), self.mask);
        self.prende(0, c, m, lim);
        self.mask = 0;
        self.fase = 0;
        self.passa();
        self.tocca_cpu();
    }

    /// Chiude la mossa: cambia turno, distribuisce o chiude il round.
    fn passa(&mut self) {
        if self.mano[0].n == 0 && self.mano[1].n == 0 {
            if self.mazzo.n > 0 {
                self.distribuisci();
            } else {
                self.fine_round();
                return;
            }
        }
        self.turno = 1 - self.turno;
    }

    fn tocca_cpu(&mut self) {
        if self.fase == 0 && self.turno == 1 {
            self.cpu_mossa();
        }
    }

    /// CPU: cattura con più carte (poi valore più alto), altrimenti scarta la più bassa.
    fn cpu_mossa(&mut self) {
        if self.mano[1].n == 0 {
            return;
        }
        let mut bi = usize::MAX;
        let mut bk = (0u32, 0u32);
        let mut bm = 0u64;
        for i in 0..self.mano[1].n {
            let v = self.mano[1].c[i].r as u32;
            let (n, m) = self.cattura(v);
            if n > 0 && (n, v) > bk {
                bk = (n, v);
                bi = i;
                bm = m;
            }
        }
        if bi != usize::MAX {
            let c = self.mano[1].remove(bi);
            let lim = self.lim();
            self.prende(1, c, bm, lim);
        } else {
            let mut mi = 0;
            for i in 1..self.mano[1].n {
                if self.mano[1].c[i].r < self.mano[1].c[mi].r {
                    mi = i;
                }
            }
            let c = self.mano[1].remove(mi);
            self.tav.push(c);
        }
        self.passa();
    }

    /// Fine round: il tavolo va all'ultimo a prendere, poi punteggio.
    fn fine_round(&mut self) {
        match self.ultimo {
            Some(u) => {
                while let Some(c) = self.tav.pop() {
                    self.prese[u].push(c);
                }
            }
            None => self.tav.clear(),
        }
        let mut rb = [0u32; 2];
        bonus(self.prese[0].n as u32, self.prese[1].n as u32, &mut rb);
        bonus(denari(&self.prese[0]), denari(&self.prese[1]), &mut rb);
        bonus(primiera(&self.prese[0]), primiera(&self.prese[1]), &mut rb);
        if ha_sette(&self.prese[0]) {
            rb[0] += 1;
        }
        if ha_sette(&self.prese[1]) {
            rb[1] += 1;
        }
        for k in 0..2 {
            self.pts[k] += rb[k];
        }
        self.rb = rb;
        self.fase = if self.pts[0] >= 11 || self.pts[1] >= 11 { 3 } else { 2 };
    }
}

impl Game for Scopa {
    fn act(&mut self, a: u32) {
        match kind(a) {
            0xFF => *self = Self::new(self.rng.next()),
            1 if self.fase == 0 && self.turno == 0 => self.gioca(arg1(a) as usize),
            2 if self.fase == 1 => {
                let j = arg1(a) as usize;
                if j < self.lim() {
                    self.mask ^= 1u64 << j;
                }
            }
            3 if self.fase == 1 => self.prendi(),
            4 if self.fase == 1 => {
                self.fase = 0;
                self.mask = 0;
            }
            5 if self.fase == 2 => {
                self.primo = 1 - self.primo;
                self.apri_round();
            }
            _ => {}
        }
    }

    fn draw(&self, p: &mut Pic, ui: &mut Ui) {
        p.clear(GREEN);
        let st = match self.fase {
            0 => "Tocca a te: gioca una carta",
            1 => "Scegli le carte da prendere",
            2 => "Round finito: premi NUOVO TURNO",
            _ => {
                if self.pts[0] > self.pts[1] {
                    "Hai vinto la partita"
                } else if self.pts[0] < self.pts[1] {
                    "Ha vinto la CPU"
                } else {
                    "Pareggio"
                }
            }
        };
        topbar(p, ui, "SCOPA", st);

        // Mano CPU coperta, in alto a sinistra
        for i in 0..self.mano[1].n {
            p.card(60 + i as i32 * 170, 80, self.mano[1].c[i], true, false, false);
        }

        // Punteggi e statistiche
        p.text(800, 90, "CPU", 26, WHITE);
        let mut x = kv(p, 920, 90, "PUNTI", self.pts[1] as i32);
        x = kv(p, x, 90, "PRESE", self.prese[1].n as i32);
        kv(p, x, 90, "SCOPE", self.scope[1] as i32);
        p.text(800, 150, "TU", 26, WHITE);
        let mut x = kv(p, 920, 150, "PUNTI", self.pts[0] as i32);
        x = kv(p, x, 150, "PRESE", self.prese[0].n as i32);
        kv(p, x, 150, "SCOPE", self.scope[0] as i32);
        let mut x = kv(p, 800, 210, "MAZZO", self.mazzo.n as i32);
        if self.fase >= 2 {
            x = kv(p, x, 210, "ROUND TU", self.rb[0] as i32);
            kv(p, x, 210, "CPU", self.rb[1] as i32);
        }

        // Tavolo: due file da 12 carte
        for j in 0..self.tav.n.min(24) {
            let (x, y) = (40 + (j % 12) as i32 * 150, 300 + (j / 12) as i32 * 200);
            let sel = self.fase == 1 && (self.mask >> j) & 1 == 1;
            p.card(x, y, self.tav.c[j], true, true, sel);
            if self.fase == 1 && j < self.lim() {
                ui.hit(x, y, CW, CH, act(2, j as u32, 0));
            }
        }

        // Mano tua
        for i in 0..self.mano[0].n {
            let (x, y) = (60 + i as i32 * 170, 720);
            p.card(x, y, self.mano[0].c[i], true, true, self.fase == 1 && i == self.gi);
            if self.fase == 0 && self.turno == 0 {
                ui.hit(x, y, CW, CH, act(1, i as u32, 0));
            }
        }

        // Pulsanti
        let f1 = self.fase == 1;
        let ok = f1 && self.mask != 0 && self.somma_mask() == self.mano[0].c[self.gi].r as u32;
        ui.button(p, 1000, 900, 260, 70, act(3, 0, 0), "PRENDI", ok);
        ui.button(p, 1290, 900, 260, 70, act(4, 0, 0), "ANNULLA", f1);
        ui.button(p, 1580, 900, 300, 70, act(5, 0, 0), "NUOVO TURNO", self.fase == 2);
    }
}

// ---------------------------------------------------------------- BRISCOLA / TRESSETTE

/// Forza per la presa (briscola: 1>3>10>9>8>7>6>5>4>2; tressette: 3>2>1>10>...>4).
fn forza(r: u8, brisc: bool) -> u32 {
    if brisc {
        match r {
            1 => 9,
            3 => 8,
            10 => 7,
            9 => 6,
            8 => 5,
            7 => 4,
            6 => 3,
            5 => 2,
            4 => 1,
            _ => 0,
        }
    } else {
        match r {
            3 => 9,
            2 => 8,
            1 => 7,
            10 => 6,
            9 => 5,
            8 => 4,
            7 => 3,
            6 => 2,
            5 => 1,
            _ => 0,
        }
    }
}

/// Punti della carta (briscola: 11/10/4/3/2; tressette: asso 3, tre/re/cavallo/fante 1).
fn punti(c: Card, brisc: bool) -> u32 {
    if brisc {
        match c.r {
            1 => 11,
            3 => 10,
            10 => 4,
            9 => 3,
            8 => 2,
            _ => 0,
        }
    } else {
        match c.r {
            1 => 3,
            3 | 8 | 9 | 10 => 1,
            _ => 0,
        }
    }
}

fn pesca(m: &mut Pile<52>, h: &mut Pile<10>) {
    if let Some(c) = m.pop() {
        h.push(c);
    }
}

/// Briscola (`brisc` = true) o Tressette (2 giocatori contro il banco).
pub struct Trick {
    rng: Rng,
    brisc: bool,
    mazzo: Pile<52>,
    trump: Card,
    mp: Pile<10>, // mano giocatore
    mb: Pile<10>, // mano banco
    tp: Option<Card>, // carta del giocatore sul tavolo
    tc: Option<Card>, // carta del banco sul tavolo
    lead_p: bool,     // il giocatore guida la presa
    gp: Pile<40>,     // prese del giocatore
    gc: Pile<40>,     // prese del banco
    fine: bool,
}

impl Trick {
    /// Briscola.
    pub fn new(seed: u32) -> Self {
        Self::with(true, seed)
    }

    /// Tressette.
    pub fn tressette(seed: u32) -> Self {
        Self::with(false, seed)
    }

    fn with(brisc: bool, seed: u32) -> Self {
        let mut t = Trick {
            rng: Rng::new(seed),
            brisc,
            mazzo: deck(true),
            trump: Card { r: 1, s: 0 },
            mp: Pile::EMPTY,
            mb: Pile::EMPTY,
            tp: None,
            tc: None,
            lead_p: true,
            gp: Pile::EMPTY,
            gc: Pile::EMPTY,
            fine: false,
        };
        shuffle(&mut t.mazzo, &mut t.rng);
        t.trump = t.mazzo.c[0]; // fondo del mazzo: ultima carta pescata
        let k = if brisc { 3 } else { 10 };
        for _ in 0..k {
            pesca(&mut t.mazzo, &mut t.mp);
            pesca(&mut t.mazzo, &mut t.mb);
        }
        if !brisc {
            t.mazzo.clear(); // 20 carte fuori gioco
        }
        t
    }

    /// Vero se la carta `a` (guidata) è battuta da `b` (risposta).
    fn batte(&self, a: Card, b: Card) -> bool {
        if self.brisc {
            let (ta, tb) = (a.s == self.trump.s, b.s == self.trump.s);
            if ta != tb {
                return tb;
            }
        }
        a.s == b.s && forza(b.r, self.brisc) > forza(a.r, self.brisc)
    }

    /// Il giocatore può giocare la carta `i` ora?
    fn puo_giocare(&self, i: usize) -> bool {
        if self.fine || self.tp.is_some() || (self.tc.is_none() != self.lead_p) || i >= self.mp.n {
            return false;
        }
        // Tressette: obbligo di seguire il seme
        if !self.brisc && !self.lead_p {
            if let Some(g) = self.tc {
                let c = self.mp.c[i];
                if c.s != g.s && self.mp.s().iter().any(|x| x.s == g.s) {
                    return false;
                }
            }
        }
        true
    }

    /// Scelta del banco: risponde a `g` (o guida se None). Vince la più economica, altrimenti la più economica.
    fn scegli(&self, g: Option<Card>) -> usize {
        let obbl = !self.brisc && g.map_or(false, |g| self.mb.s().iter().any(|c| c.s == g.s));
        let mut bi = 0;
        let mut bk = (u32::MAX, u32::MAX, u32::MAX);
        for i in 0..self.mb.n {
            let c = self.mb.c[i];
            if obbl && g.map_or(false, |g| c.s != g.s) {
                continue;
            }
            let vince = g.map_or(false, |g| self.batte(g, c));
            let k = (if vince { 0u32 } else { 1u32 }, punti(c, self.brisc), forza(c.r, self.brisc));
            if k < bk {
                bk = k;
                bi = i;
            }
        }
        bi
    }

    /// Risolve la presa quando entrambe le carte sono sul tavolo.
    fn risolvi(&mut self) {
        let (Some(tp), Some(tc)) = (self.tp, self.tc) else {
            return;
        };
        let (a, b) = if self.lead_p { (tp, tc) } else { (tc, tp) };
        let vb = self.batte(a, b);
        let vince_p = if self.lead_p { !vb } else { vb };
        if vince_p {
            self.gp.push(a);
            self.gp.push(b);
        } else {
            self.gc.push(a);
            self.gc.push(b);
        }
        self.tp = None;
        self.tc = None;
        self.lead_p = vince_p;
        if self.brisc {
            if vince_p {
                pesca(&mut self.mazzo, &mut self.mp);
                pesca(&mut self.mazzo, &mut self.mb);
            } else {
                pesca(&mut self.mazzo, &mut self.mb);
                pesca(&mut self.mazzo, &mut self.mp);
            }
        }
        if self.mp.n == 0 && self.mb.n == 0 {
            self.fine = true;
            return;
        }
        if !self.lead_p {
            let k = self.scegli(None);
            self.tc = Some(self.mb.remove(k));
        }
    }

    fn sum(&self, p: &Pile<40>) -> u32 {
        p.s().iter().fold(0u32, |a, c| a + punti(*c, self.brisc))
    }
}

impl Game for Trick {
    fn act(&mut self, a: u32) {
        if kind(a) == 0xFF {
            *self = Self::with(self.brisc, self.rng.next());
            return;
        }
        if kind(a) == 1 && self.puo_giocare(arg1(a) as usize) {
            let c = self.mp.remove(arg1(a) as usize);
            self.tp = Some(c);
            if self.lead_p {
                // il banco risponde subito
                let k = self.scegli(self.tp);
                self.tc = Some(self.mb.remove(k));
            }
            self.risolvi();
        }
    }

    fn draw(&self, p: &mut Pic, ui: &mut Ui) {
        p.clear(GREEN);
        let st = if self.fine {
            let (pp, pc) = (self.sum(&self.gp), self.sum(&self.gc));
            if pp == pc {
                "Pareggio"
            } else if pp > pc {
                "Hai vinto la partita"
            } else {
                "Ha vinto il banco"
            }
        } else {
            "Tocca a te: scegli una carta"
        };
        topbar(p, ui, if self.brisc { "BRISCOLA" } else { "TRESSETTE" }, st);

        // Mano banco coperta, in alto
        for i in 0..self.mb.n {
            p.card(60 + i as i32 * 170, 80, self.mb.c[i], true, false, false);
        }

        // Briscola a sinistra
        if self.brisc {
            p.card(40, 330, self.trump, true, true, false);
            p.text(40, 535, "BRISCOLA", 22, WHITE);
            if self.mazzo.n > 0 {
                p.card(230, 330, self.trump, true, false, false);
                num(p, 230, 535, self.mazzo.n as i32, 26, YEL);
            }
        } else {
            p.text(40, 330, "TRESSETTE", 26, WHITE);
            p.text(40, 370, "20 CARTE FUORI", 22, WHITE);
        }

        // Tavolo
        if let Some(c) = self.tc {
            p.card(700, 330, c, true, true, false);
        }
        if let Some(c) = self.tp {
            p.card(1000, 330, c, true, true, false);
        }
        p.text(700, 535, "BANCO", 24, WHITE);
        p.text(1000, 535, "TU", 24, WHITE);

        // Punti
        kv(p, 1400, 330, "BANCO", self.sum(&self.gc) as i32);
        kv(p, 1400, 390, "TU", self.sum(&self.gp) as i32);

        // Mano giocatore
        for i in 0..self.mp.n {
            let (x, y) = (60 + i as i32 * 170, 720);
            p.card(x, y, self.mp.c[i], true, true, false);
            if self.puo_giocare(i) {
                ui.hit(x, y, CW, CH, act(1, i as u32, 0));
            }
        }
    }
}

// ---------------------------------------------------------------- SETTE E MEZZO

const PUNTATA: i32 = 10;

/// Valore in metà punti: 1..7 = 2*rango, 8/9/10 = 1 (0,5).
fn mezzi(c: Card) -> u32 {
    if c.r <= 7 { 2 * c.r as u32 } else { 1 }
}

fn somma(p: &Pile<40>) -> u32 {
    p.s().iter().fold(0u32, |a, c| a + mezzi(*c))
}

/// Sette e mezzo contro il banco. Puntata 10; vittoria riscuote 20.
pub struct SetteMezzo {
    rng: Rng,
    mazzo: Pile<52>,
    tu: Pile<40>,
    banco: Pile<40>,
    fiches: i32,
    fine: bool,
    msg: &'static str,
}

impl SetteMezzo {
    pub fn new(seed: u32) -> Self {
        let mut s = SetteMezzo {
            rng: Rng::new(seed),
            mazzo: deck(true),
            tu: Pile::EMPTY,
            banco: Pile::EMPTY,
            fiches: 100,
            fine: true,
            msg: "",
        };
        shuffle(&mut s.mazzo, &mut s.rng);
        s.nuova_mano();
        s
    }

    /// Nuova mano: punta 10 e distribuisce una carta a testa.
    fn nuova_mano(&mut self) {
        if self.mazzo.n < 10 {
            self.mazzo = deck(true);
            shuffle(&mut self.mazzo, &mut self.rng);
        }
        self.tu.clear();
        self.banco.clear();
        self.fiches -= PUNTATA;
        self.fine = false;
        self.msg = "Chiedi carta o stai";
        if let Some(c) = self.mazzo.pop() {
            self.tu.push(c);
        }
        if let Some(c) = self.mazzo.pop() {
            self.banco.push(c);
        }
    }

    fn chiudi(&mut self, vinto: bool, msg: &'static str) {
        self.fine = true;
        if vinto {
            self.fiches += 2 * PUNTATA;
        }
        self.msg = msg;
    }

    /// Il banco pesca fino a 11 metà punti (5,5), poi confronto.
    fn stai(&mut self) {
        while somma(&self.banco) < 11 {
            match self.mazzo.pop() {
                Some(c) => self.banco.push(c),
                None => break,
            }
        }
        let (t, b) = (somma(&self.tu), somma(&self.banco));
        if b > 15 {
            self.chiudi(true, "Il banco sballa: hai vinto");
        } else if t > b {
            self.chiudi(true, "Hai vinto");
        } else {
            self.chiudi(false, "Vince il banco (parità inclusa)");
        }
    }
}

impl Game for SetteMezzo {
    fn act(&mut self, a: u32) {
        match kind(a) {
            0xFF => *self = Self::new(self.rng.next()),
            1 if !self.fine => {
                if let Some(c) = self.mazzo.pop() {
                    self.tu.push(c);
                }
                if somma(&self.tu) > 15 {
                    self.chiudi(false, "Hai sballato: perdi la puntata");
                }
            }
            2 if !self.fine => self.stai(),
            3 if self.fine && self.fiches >= PUNTATA => self.nuova_mano(),
            _ => {}
        }
    }

    fn draw(&self, p: &mut Pic, ui: &mut Ui) {
        p.clear(GREEN);
        topbar(p, ui, "SETTE E MEZZO", self.msg);

        // Banco: coperto finché la mano non finisce
        p.text(1500, 100, "BANCO", 26, WHITE);
        for i in 0..self.banco.n {
            p.card(60 + i as i32 * 150, 100, self.banco.c[i], true, self.fine, false);
        }
        if self.fine {
            sum_text(p, 1500, 150, somma(&self.banco), 48, WHITE);
        }

        // Tu
        for i in 0..self.tu.n {
            p.card(60 + i as i32 * 150, 500, self.tu.c[i], true, true, false);
        }
        p.text(1500, 500, "TU", 26, WHITE);
        sum_text(p, 1500, 550, somma(&self.tu), 48, YEL);

        // Fiches e pulsanti
        p.text(60, 760, "FICHES", 30, WHITE);
        num(p, 280, 760, self.fiches, 30, YEL);
        p.text(600, 760, "PUNTATA 10", 30, WHITE);
        p.text(1100, 760, "LIMITE 7 E MEZZO", 30, WHITE);
        ui.button(p, 60, 880, 260, 90, act(1, 0, 0), "CARTA", !self.fine);
        ui.button(p, 360, 880, 260, 90, act(2, 0, 0), "STAI", !self.fine);
        ui.button(p, 660, 880, 360, 90, act(3, 0, 0), "NUOVA MANO", self.fine && self.fiches >= PUNTATA);
    }
}
