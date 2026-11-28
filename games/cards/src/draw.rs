//! Disegno vettoriale su buffer ARGB. Coordinate logiche 1920x1080, scalate sul buffer.
use crate::{font, Card, NEW};

pub const W: i32 = 1920;
pub const H: i32 = 1080;
pub const CW: i32 = 130;
pub const CH: i32 = 190;
pub const GREEN: u32 = 0xFF1B5E3B;
pub const DARK: u32 = 0xFF0F3D26;
pub const WHITE: u32 = 0xFFF8F8F0;
pub const INK: u32 = 0xFF222222;
pub const RED: u32 = 0xFFC62828;
pub const GOLD: u32 = 0xFFD4A017;
pub const YEL: u32 = 0xFFFFD60A;
pub const BLUE: u32 = 0xFF1F3A93;
pub const GRN: u32 = 0xFF2E7D32;
pub const BACK: u32 = 0xFF1E3A8A;
pub const GREY: u32 = 0xFF555555;
pub const BTN: u32 = 0xFF2D6A4F;

pub struct Pic<'a> {
    pub px: &'a mut [u32],
    pub w: i32,
    pub h: i32,
}

/// Larghezza stimata di una riga di testo di altezza `h`.
pub fn text_w(s: &str, h: i32) -> i32 {
    s.len() as i32 * 6 * (h / 6).max(1)
}

/// Converte un intero in testo senza allocazione.
pub fn itoa(mut n: i32, b: &mut [u8; 12]) -> &str {
    let neg = n < 0;
    if neg {
        n = -n;
    }
    let mut i = b.len();
    loop {
        i -= 1;
        b[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 || i == 0 {
            break;
        }
    }
    if neg && i > 0 {
        i -= 1;
        b[i] = b'-';
    }
    core::str::from_utf8(&b[i..]).unwrap_or("")
}

pub fn rank_str(r: u8, napoli: bool) -> &'static str {
    const F: [&str; 14] = ["", "A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"];
    const N: [&str; 11] = ["", "A", "2", "3", "4", "5", "6", "7", "F", "C", "R"];
    if napoli { N[r as usize] } else { F[r as usize] }
}

pub fn suit_color(napoli: bool, s: u8) -> u32 {
    if napoli {
        [RED, 0xFFB8860B, BLUE, GRN][s as usize & 3]
    } else {
        [INK, RED, RED, INK][s as usize & 3]
    }
}

impl Pic<'_> {
    fn dx(&self, x: i32) -> i32 {
        x * self.w / W
    }
    fn dy(&self, y: i32) -> i32 {
        y * self.h / H
    }
    fn put(&mut self, x: i32, y: i32, c: u32) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = c;
        }
    }

    /// Sfondo a gradiente verticale: da `c` al colore scuro (feltro con vignetta).
    pub fn clear(&mut self, c: u32) {
        let dark = mix(c, 0xFF000000, 120);
        for y in 0..self.h {
            let row = mix(c, dark, (y * 255 / self.h.max(1)) as u32);
            let base = (y * self.w) as usize;
            self.px[base..base + self.w as usize].iter_mut().for_each(|p| *p = row);
        }
    }

    /// Rettangolo semitrasparente (alpha 0..255), mescolato con il buffer.
    pub fn rect_a(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32, a: u32) {
        for yy in self.dy(y)..self.dy(y + h) {
            for xx in self.dx(x)..self.dx(x + w) {
                if xx >= 0 && yy >= 0 && xx < self.w && yy < self.h {
                    let i = (yy * self.w + xx) as usize;
                    self.px[i] = mix(self.px[i], c, a);
                }
            }
        }
    }

    /// Ombra morbida sotto una carta (strati sfumati).
    pub fn shadow(&mut self, x: i32, y: i32, w: i32, h: i32) {
        for k in 0..6 {
            let e = k * 3;
            self.rect_a(x - e + 6, y - e + 10, w + 2 * e, h + 2 * e, 0xFF000000, 26);
        }
    }

    /// Alone luminoso attorno a un riquadro (selezione).
    pub fn glow(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        for k in 1..=5 {
            let e = k * 4;
            self.rect_a(x - e, y - e, w + 2 * e, h + 2 * e, c, 40);
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        for yy in self.dy(y)..self.dy(y + h) {
            for xx in self.dx(x)..self.dx(x + w) {
                self.put(xx, yy, c);
            }
        }
    }

    pub fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, t: i32, c: u32) {
        self.rect(x, y, w, t, c);
        self.rect(x, y + h - t, w, t, c);
        self.rect(x, y, t, h, c);
        self.rect(x + w - t, y, t, h, c);
    }

    pub fn circle(&mut self, cx: i32, cy: i32, r: i32, c: u32) {
        let (dcx, dcy) = (self.dx(cx) as i64, self.dy(cy) as i64);
        let rx = self.dx(r).max(1) as i64;
        let ry = self.dy(r).max(1) as i64;
        for yy in (dcy - ry)..=(dcy + ry) {
            for xx in (dcx - rx)..=(dcx + rx) {
                let (u, v) = (xx - dcx, yy - dcy);
                if u * u * ry * ry + v * v * rx * rx <= rx * rx * ry * ry {
                    self.put(xx as i32, yy as i32, c);
                }
            }
        }
    }

    /// Poligono convesso (fino a 8 vertici, ordine orario o antiorario).
    pub fn poly(&mut self, pts: &[(i32, i32)], c: u32) {
        let n = pts.len().min(8);
        let mut d = [(0i64, 0i64); 8];
        for i in 0..n {
            d[i] = (self.dx(pts[i].0) as i64, self.dy(pts[i].1) as i64);
        }
        let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
        for p in &d[..n] {
            x0 = x0.min(p.0);
            y0 = y0.min(p.1);
            x1 = x1.max(p.0);
            y1 = y1.max(p.1);
        }
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (mut pos, mut neg) = (false, false);
                for i in 0..n {
                    let a = d[i];
                    let b = d[(i + 1) % n];
                    let cr = (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0);
                    if cr > 0 {
                        pos = true;
                    } else if cr < 0 {
                        neg = true;
                    }
                }
                if !(pos && neg) {
                    self.put(x as i32, y as i32, c);
                }
            }
        }
    }

    /// Segmento di spessore `t` (coordinate logiche).
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, t: i32, c: u32) {
        let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
        for i in 0..=n {
            let x = x0 + (x1 - x0) * i / n;
            let y = y0 + (y1 - y0) * i / n;
            self.rect(x - t / 2, y - t / 2, t, t, c);
        }
    }

    /// Testo vettoriale a tratti. `h` = altezza del carattere.
    pub fn text(&mut self, x: i32, y: i32, s: &str, h: i32, c: u32) {
        let u = (h / 6).max(1);
        let t = (h / 9).max(2);
        let mut cx = x;
        for b in s.bytes() {
            for stroke in font::glyph(b.to_ascii_uppercase()).split('|') {
                let mut prev: Option<(i32, i32)> = None;
                for pt in stroke.split(' ') {
                    let p = pt.as_bytes();
                    if p.len() < 3 {
                        continue;
                    }
                    let q = (cx + (p[0] - b'0') as i32 * u, y + (p[2] - b'0') as i32 * u);
                    if let Some(a) = prev {
                        self.line(a.0, a.1, q.0, q.1, t, c);
                    }
                    prev = Some(q);
                }
            }
            cx += 6 * u;
        }
    }

    /// Testo centrato in un riquadro.
    pub fn ctext(&mut self, x: i32, y: i32, w: i32, h: i32, s: &str, sz: i32, c: u32) {
        let tw = text_w(s, sz);
        self.text(x + (w - tw) / 2, y + (h - sz) / 2, s, sz, c);
    }

    /// Seme vettoriale centrato in (cx, cy), raggio `r`.
    pub fn suit(&mut self, cx: i32, cy: i32, r: i32, napoli: bool, s: u8) {
        let c = suit_color(napoli, s);
        if napoli {
            match s & 3 {
                0 => {
                    // coppe
                    self.poly(&[(cx - r, cy - r), (cx + r, cy - r), (cx + r / 2, cy + r / 4), (cx - r / 2, cy + r / 4)], c);
                    self.rect(cx - r / 6, cy + r / 4, r / 3, r / 2, c);
                    self.rect(cx - r / 2, cy + r * 3 / 4, r, r / 4, c);
                }
                1 => {
                    // denari
                    self.circle(cx, cy, r, c);
                    self.circle(cx, cy, r * 3 / 4, 0xFFFFE57F);
                    self.circle(cx, cy, r / 3, c);
                }
                2 => {
                    // spade
                    self.line(cx - r, cy - r, cx + r, cy + r, r / 4 + 1, c);
                    self.line(cx + r, cy - r, cx - r, cy + r, r / 4 + 1, c);
                }
                _ => {
                    // bastoni
                    self.rect(cx - r / 4, cy - r, r / 2, 2 * r, c);
                    self.circle(cx, cy - r, r / 3, c);
                    self.circle(cx, cy + r, r / 3, c);
                }
            }
        } else {
            match s & 3 {
                0 => {
                    // picche
                    self.circle(cx - r / 2, cy + r / 5, r / 2, c);
                    self.circle(cx + r / 2, cy + r / 5, r / 2, c);
                    self.poly(&[(cx, cy - r), (cx + r, cy + r / 5), (cx - r, cy + r / 5)], c);
                    self.poly(&[(cx - r / 6, cy + r / 5), (cx + r / 6, cy + r / 5), (cx + r / 3, cy + r), (cx - r / 3, cy + r)], c);
                }
                1 => {
                    // cuori
                    self.circle(cx - r / 2, cy - r / 5, r / 2, c);
                    self.circle(cx + r / 2, cy - r / 5, r / 2, c);
                    self.poly(&[(cx - r, cy - r / 5), (cx + r, cy - r / 5), (cx, cy + r)], c);
                }
                2 => {
                    // quadri
                    self.poly(&[(cx, cy - r), (cx + r * 7 / 10, cy), (cx, cy + r), (cx - r * 7 / 10, cy)], c);
                }
                _ => {
                    // fiori
                    self.circle(cx, cy - r / 2, r * 2 / 5, c);
                    self.circle(cx - r / 2, cy + r / 5, r * 2 / 5, c);
                    self.circle(cx + r / 2, cy + r / 5, r * 2 / 5, c);
                    self.poly(&[(cx - r / 6, cy + r / 4), (cx + r / 6, cy + r / 4), (cx + r / 3, cy + r), (cx - r / 3, cy + r)], c);
                }
            }
        }
    }

    /// Carta CW x CH. `up` = faccia in su, `sel` = bordo evidenziato.
    pub fn card(&mut self, x: i32, y: i32, c: Card, napoli: bool, up: bool, sel: bool) {
        if sel {
            self.glow(x, y, CW, CH, YEL);
        }
        self.shadow(x, y, CW, CH);
        if !up {
            self.rect(x, y, CW, CH, BACK);
            self.frame(x + 8, y + 8, CW - 16, CH - 16, 3, 0xFF8AA4E0);
            self.rect_a(x + 14, y + 14, CW - 28, CH - 28, 0xFFFFFFFF, 40);
            return;
        }
        self.rect(x, y, CW, CH, WHITE);
        self.frame(x, y, CW, CH, 2, INK);
        let col = suit_color(napoli, c.s);
        self.text(x + 10, y + 10, rank_str(c.r, napoli), 30, col);
        self.suit(x + CW / 2, y + CH / 2, 36, napoli, c.s);
        self.suit(x + 20, y + 58, 10, napoli, c.s);
    }
}

/// Mescola due colori ARGB: `a` = peso di `s` (0..255).
pub fn mix(d: u32, s: u32, a: u32) -> u32 {
    let ch = |sh: u32| {
        let (x, y) = ((d >> sh) & 255, (s >> sh) & 255);
        ((x * (255 - a) + y * a) / 255) << sh
    };
    0xFF00_0000 | ch(16) | ch(8) | ch(0)
}

/// Numero con eventuale ".5" (usato dal Sette e mezzo, valori in metà punti).
pub fn sum_text(p: &mut Pic, x: i32, y: i32, half: u32, h: i32, c: u32) {
    let mut b = [0u8; 12];
    let s = itoa((half / 2) as i32, &mut b);
    p.text(x, y, s, h, c);
    if half % 2 == 1 {
        let w = text_w(s, h);
        p.text(x + w, y, ".5", h, c);
    }
}

#[derive(Clone, Copy)]
pub struct Hit {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    a: u32,
}

/// Punti cliccabili del frame corrente. Si ricostruisce a ogni `draw`.
/// Tastiera: `next`/`prev` cambiano il fuoco, `fire` dà l'azione del fuoco.
pub struct Ui {
    hits: [Hit; 128],
    n: usize,
    focus: usize,
}

impl Ui {
    pub const fn new() -> Self {
        Self { hits: [Hit { x: 0, y: 0, w: 0, h: 0, a: 0 }; 128], n: 0, focus: 0 }
    }
    pub fn reset(&mut self) {
        self.n = 0;
    }
    pub fn hit(&mut self, x: i32, y: i32, w: i32, h: i32, a: u32) {
        if self.n < self.hits.len() {
            self.hits[self.n] = Hit { x, y, w, h, a };
            self.n += 1;
        }
    }
    pub fn button(&mut self, p: &mut Pic, x: i32, y: i32, w: i32, h: i32, a: u32, label: &str, on: bool) {
        p.rect(x, y, w, h, if on { BTN } else { GREY });
        p.frame(x, y, w, h, 3, WHITE);
        let n = label.len().max(1) as i32;
        let sz = (h / 2).min(40).min((w - 16) / n).max(8);
        p.ctext(x, y, w, h, label, sz, WHITE);
        if on {
            self.hit(x, y, w, h, a);
        }
    }
    /// Azione sotto il punto (coordinate logiche); vince il punto disegnato per ultimo.
    pub fn click(&self, x: i32, y: i32) -> Option<u32> {
        (0..self.n).rev().map(|i| self.hits[i]).find(|h| x >= h.x && x < h.x + h.w && y >= h.y && y < h.y + h.h).map(|h| h.a)
    }
    pub fn next(&mut self) {
        if self.n > 0 {
            self.focus = (self.focus + 1) % self.n;
        }
    }
    pub fn prev(&mut self) {
        if self.n > 0 {
            self.focus = (self.focus + self.n - 1) % self.n;
        }
    }
    pub fn fire(&self) -> Option<u32> {
        if self.focus < self.n { Some(self.hits[self.focus].a) } else { None }
    }
    pub fn focus_frame(&self, p: &mut Pic) {
        if self.focus < self.n {
            let h = self.hits[self.focus];
            p.frame(h.x - 4, h.y - 4, h.w + 8, h.h + 8, 4, YEL);
        }
    }
}

/// Barra superiore: titolo, stato e pulsante nuova partita.
pub fn topbar(p: &mut Pic, ui: &mut Ui, title: &str, status: &str) {
    p.rect(0, 0, W, 64, DARK);
    p.text(30, 16, title, 32, WHITE);
    p.text(420, 20, status, 26, YEL);
    ui.button(p, W - 300, 12, 260, 44, NEW, "NUOVA PARTITA", true);
}
