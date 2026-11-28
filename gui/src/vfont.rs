//! Font vettoriale con antialiasing. no_std, senza allocazione, senza dipendenze.
//! Contorni in unità 0..=64 per em, y verso il basso. Ops (byte):
//!   0 x y         move (apre un contorno)
//!   1 x y         linea
//!   2 cx cy x y   curva quadratica (4 segmenti)
//!   3             chiude il contorno
//!   4 x0 y0 x1 y1 rettangolo (scorciatoia per move+3 linee+chiusura)
//! Riempimento non zero. Copertura: 4 sotto-scanline per riga, area orizzontale esatta.
//! Limiti: MAX_EDGES lati per glifo (i lati in eccesso sono scartati), size ≤ MAX_SIZE.
use crate::palette::Tok;
use crate::render::Canvas;

pub const EM: i32 = 64;
/// Passo tra due caratteri, in unità (glifo 5 colonne + 1 vuota, cella 8).
pub const ADVANCE: i32 = 48;
pub const MAX_SIZE: i32 = 64;
const SUB: i32 = 4;
const MAX_EDGES: usize = 128;
const CURVE_STEPS: i64 = 4;
const ROW_W: usize = MAX_SIZE as usize + 2;

#[derive(Clone, Copy, Default)]
struct Edge {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

/// Lati di un glifo in coordinate fisse (1/256 px, assolute sul canvas).
struct Edges {
    e: [Edge; MAX_EDGES],
    n: usize,
    ox: i32,
    oy: i32,
    k: i32,
}

impl Edges {
    fn p(&self, x: u8, y: u8) -> (i32, i32) {
        (self.ox + x as i32 * self.k, self.oy + y as i32 * self.k)
    }

    fn push(&mut self, a: (i32, i32), b: (i32, i32)) {
        if a.1 == b.1 || self.n == MAX_EDGES {
            return;
        }
        self.e[self.n] = Edge { x0: a.0, y0: a.1, x1: b.0, y1: b.1 };
        self.n += 1;
    }

    fn quad(&mut self, p0: (i32, i32), c: (i32, i32), p1: (i32, i32)) {
        let mut prev = p0;
        for s in 1..=CURVE_STEPS {
            let u = CURVE_STEPS - s;
            let n2 = CURVE_STEPS * CURVE_STEPS;
            let x = (u * u * p0.0 as i64 + 2 * u * s * c.0 as i64 + s * s * p1.0 as i64) / n2;
            let y = (u * u * p0.1 as i64 + 2 * u * s * c.1 as i64 + s * s * p1.1 as i64) / n2;
            let p = (x as i32, y as i32);
            self.push(prev, p);
            prev = p;
        }
    }
}

/// Converte gli ops di un glifo in lati, con origine `(gx, gy)` px e `size` px per em.
fn build(ops: &[u8], gx: i32, gy: i32, size: i32) -> Edges {
    let mut ed = Edges {
        e: [Edge::default(); MAX_EDGES],
        n: 0,
        ox: gx * 256,
        oy: gy * 256,
        k: size * 4,
    };
    let (mut start, mut cur, mut open) = ((0, 0), (0, 0), false);
    let mut i = 0;
    while i < ops.len() {
        let need = match ops[i] {
            0 | 1 => 3,
            2 | 4 => 5,
            3 => 1,
            _ => break,
        };
        if i + need > ops.len() {
            break;
        }
        let a = &ops[i + 1..i + need];
        match ops[i] {
            0 => {
                if open {
                    ed.push(cur, start);
                }
                start = ed.p(a[0], a[1]);
                cur = start;
                open = true;
            }
            1 => {
                let p = ed.p(a[0], a[1]);
                ed.push(cur, p);
                cur = p;
            }
            2 => {
                let c = ed.p(a[0], a[1]);
                let p = ed.p(a[2], a[3]);
                ed.quad(cur, c, p);
                cur = p;
            }
            3 => {
                if open {
                    ed.push(cur, start);
                }
                cur = start;
                open = false;
            }
            _ => {
                let (p0, p1) = (ed.p(a[0], a[1]), ed.p(a[2], a[3]));
                let (q, r, s) = ((p1.0, p0.1), p1, (p0.0, p1.1));
                ed.push(p0, q);
                ed.push(q, r);
                ed.push(r, s);
                ed.push(s, p0);
            }
        }
        i += need;
    }
    if open {
        ed.push(cur, start);
    }
    ed
}

/// Somma al `acc` della riga la copertura dello span `[a, b)` (1/256 px, relativi all'origine).
fn add_span(acc: &mut [u16], a: i32, b: i32) {
    let last = ((b - 1) >> 8).min(acc.len() as i32 - 1);
    let mut i = (a >> 8).max(0);
    while i <= last {
        let lo = a.max(i << 8);
        let hi = b.min((i + 1) << 8);
        if hi > lo {
            acc[i as usize] += (hi - lo) as u16;
        }
        i += 1;
    }
}

/// Rasterizza un glifo a `(gx, gy)` px (angolo alto-sinistro dell'em) con `size` px per em.
pub fn raster<C: Canvas>(c: &mut C, ops: &[u8], gx: i32, gy: i32, size: i32, t: Tok) {
    let size = size.clamp(1, MAX_SIZE);
    let ed = build(ops, gx, gy, size);
    let base = gx * 256;
    let mut xs = [(0i32, 0i32); MAX_EDGES];
    for row in 0..size {
        let mut acc = [0u16; ROW_W];
        for s in 0..SUB {
            let y = (gy + row) * 256 + ((2 * s + 1) * 256) / (2 * SUB);
            let mut m = 0;
            for e in &ed.e[..ed.n] {
                let (lo, hi, dir) = if e.y0 < e.y1 { (e.y0, e.y1, 1) } else { (e.y1, e.y0, -1) };
                if y >= lo && y < hi {
                    let dx = (e.x1 - e.x0) as i64;
                    let dy = (e.y1 - e.y0) as i64;
                    let x = e.x0 as i64 + (y - e.y0) as i64 * dx / dy;
                    xs[m] = (x as i32, dir);
                    m += 1;
                }
            }
            for a in 1..m {
                let v = xs[a];
                let mut b = a;
                while b > 0 && xs[b - 1].0 > v.0 {
                    xs[b] = xs[b - 1];
                    b -= 1;
                }
                xs[b] = v;
            }
            let mut wind = 0;
            for j in 0..m {
                wind += xs[j].1;
                if wind != 0 && j + 1 < m {
                    add_span(&mut acc, xs[j].0 - base, xs[j + 1].0 - base);
                }
            }
        }
        for (i, &v) in acc.iter().enumerate().take((size + 2) as usize) {
            let a = (v >> 2).min(255) as u8;
            if a > 0 {
                c.blend(gx + i as i32, gy + row, a, t);
            }
        }
    }
}

/// Glifo ASCII 0x20..=0x7E; ogni altro byte è spazio.
fn glyph(ch: u8) -> &'static [u8] {
    if (0x20..=0x7E).contains(&ch) {
        crate::vfont_data::GLYPHS[(ch - 0x20) as usize]
    } else {
        crate::vfont_data::GLYPHS[0]
    }
}

/// Larghezza in px di `s` a `size` px per em (passo esatto per carattere, senza accumulo di errore).
pub fn advance(size: i32, s: &str) -> i32 {
    (s.len() as i32 * ADVANCE * size.clamp(1, MAX_SIZE)) / EM
}

/// Disegna `s` con origine `(x, y)` (alto-sinistra dell'em) e `size` px per em. Ritorna la larghezza.
pub fn draw_text<C: Canvas>(c: &mut C, x: i32, y: i32, size: i32, s: &str, t: Tok) -> i32 {
    let size = size.clamp(1, MAX_SIZE);
    for (i, ch) in s.bytes().enumerate() {
        let gx = x + (i as i32 * ADVANCE * size) / EM;
        raster(c, glyph(ch), gx, y, size, t);
    }
    advance(size, s)
}
