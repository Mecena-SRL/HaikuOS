//! Testo vettoriale TrueType con antialiasing. no_std, senza allocazione.
//! Contorni letti da `ttf.rs` dal font di sistema (`fonts.rs`), riempimento non zero, copertura
//! per sotto-scanline con area orizzontale esatta. Limiti: MAX_EDGES lati per glifo, size ≤ MAX_SIZE.
use crate::fonts;
use crate::palette::Tok;
use crate::render::Canvas;
use crate::ttf::{Font, Glyph, MAX_PTS};

pub const MAX_SIZE: i32 = 256;
const SUB: i32 = 4;
const MAX_EDGES: usize = 384;
const CURVE_STEPS: i64 = 4;
const ROW_W: usize = 2 * MAX_SIZE as usize + 2;

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
    fn new() -> Edges {
        Edges { e: [Edge::default(); MAX_EDGES], n: 0, ox: 0, oy: 0, k: 0 }
    }

    /// Punto degli ops (0..64 per em) in coordinate assolute.
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

/// Converte gli ops di un glifo (formato 0..64 per em) in lati, con origine `(gx, gy)` px.
fn build(ops: &[u8], gx: i32, gy: i32, size: i32) -> Edges {
    let mut ed = Edges::new();
    ed.ox = gx * 256;
    ed.oy = gy * 256;
    ed.k = size * 4;
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

/// Riempie i lati con antialiasing: righe e colonne derivate dal loro bounding box.
fn raster_edges<C: Canvas>(c: &mut C, ed: &Edges, t: Tok) {
    if ed.n == 0 {
        return;
    }
    let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for e in &ed.e[..ed.n] {
        x0 = x0.min(e.x0.min(e.x1));
        x1 = x1.max(e.x0.max(e.x1));
        y0 = y0.min(e.y0.min(e.y1));
        y1 = y1.max(e.y0.max(e.y1));
    }
    let ax = x0 >> 8;
    let w = (((x1 + 255) >> 8) - ax).clamp(0, ROW_W as i32) as usize;
    let mut xs = [(0i32, 0i32); MAX_EDGES];
    for row in (y0 >> 8)..((y1 + 255) >> 8) {
        let mut acc = [0u16; ROW_W];
        for s in 0..SUB {
            let y = row * 256 + ((2 * s + 1) * 256) / (2 * SUB);
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
                    add_span(&mut acc, xs[j].0 - (ax << 8), xs[j + 1].0 - (ax << 8));
                }
            }
        }
        for (i, &v) in acc.iter().enumerate().take(w) {
            let a = (v >> 2).min(255) as u8;
            if a > 0 {
                c.blend(ax + i as i32, row, a, t);
            }
        }
    }
}

/// Rasterizza un glifo in formato ops a `(gx, gy)` px (angolo alto-sinistro dell'em), `size` px per em.
pub fn raster<C: Canvas>(c: &mut C, ops: &[u8], gx: i32, gy: i32, size: i32, t: Tok) {
    let size = size.clamp(1, MAX_SIZE);
    raster_edges(c, &build(ops, gx, gy, size), t);
}

/// Lati di un glifo TrueType: origine della penna `pen` e linea di base `base` (1/256 px), `size` px per em.
fn glyph_edges(g: &Glyph, upem: i32, pen: i64, base: i64, size: i64) -> Edges {
    let mut ed = Edges::new();
    let sc = size * 256;
    let up = upem as i64;
    let mut q = [(0i32, 0i32, false); MAX_PTS];
    for i in 0..g.n {
        let p = g.pts[i];
        q[i] = ((pen + p.x as i64 * sc / up) as i32, (base - p.y as i64 * sc / up) as i32, p.on);
    }
    let mut first = 0;
    for c in 0..g.nc {
        contour(&mut ed, &q[first..=g.ends[c]]);
        first = g.ends[c] + 1;
    }
    ed
}

/// Un contorno chiuso: curve quadratiche TrueType (punti fuori curva fra due punti su curva).
fn contour(ed: &mut Edges, p: &[(i32, i32, bool)]) {
    let n = p.len();
    if n < 2 {
        return;
    }
    // Sequenza `l` che termina con un punto su curva (= inizio del contorno).
    let mut l = [(0i32, 0i32, false); MAX_PTS + 1];
    let m;
    if p[0].2 {
        for i in 0..n {
            l[i] = p[(i + 1) % n];
        }
        m = n;
    } else if p[n - 1].2 {
        l[..n].copy_from_slice(p);
        m = n;
    } else {
        l[..n].copy_from_slice(p);
        l[n] = ((p[n - 1].0 + p[0].0) / 2, (p[n - 1].1 + p[0].1) / 2, true);
        m = n + 1;
    }
    let mut prev = (l[m - 1].0, l[m - 1].1);
    let mut i = 0;
    while i < m {
        let (x, y, on) = l[i];
        if on {
            ed.push(prev, (x, y));
            prev = (x, y);
            i += 1;
        } else {
            // l[m - 1] è su curva, quindi `i + 1` esiste sempre.
            let (nx, ny, non) = l[i + 1];
            if non {
                ed.quad(prev, (x, y), (nx, ny));
                prev = (nx, ny);
                i += 2;
            } else {
                let mid = ((x + nx) / 2, (y + ny) / 2);
                ed.quad(prev, (x, y), mid);
                prev = mid;
                i += 1;
            }
        }
    }
}

fn clamp_size(size: i32) -> i64 {
    size.clamp(1, MAX_SIZE) as i64
}

/// Avanzo totale di `s` in 1/256 px a `size` px per em (kerning escluso).
fn run_width(f: &Font, size: i64, s: &str) -> i64 {
    s.chars().map(|ch| f.advance(f.glyph_id(ch as u32)) as i64 * size * 256 / f.upem as i64).sum()
}

/// Larghezza in px di `s` a `size` px per em, con il font di sistema.
pub fn advance(size: i32, s: &str) -> i32 {
    match fonts::ui() {
        Some(f) => ((run_width(&f, clamp_size(size), s) + 128) / 256) as i32,
        None => 0,
    }
}

/// Disegna `s` con origine `(x, y)` (alto del riquadro dell'em) e `size` px per em. Ritorna la larghezza.
pub fn draw_text<C: Canvas>(c: &mut C, x: i32, y: i32, size: i32, s: &str, t: Tok) -> i32 {
    let Some(f) = fonts::ui() else { return 0 };
    let size = clamp_size(size);
    let base = y as i64 * 256 + f.ascender as i64 * size * 256 / f.upem as i64;
    let mut pen = x as i64 * 256;
    for ch in s.chars() {
        let gid = f.glyph_id(ch as u32);
        if let Some(g) = f.glyph(gid) {
            raster_edges(c, &glyph_edges(&g, f.upem, pen, base, size), t);
        }
        pen += f.advance(gid) as i64 * size * 256 / f.upem as i64;
    }
    ((run_width(&f, size, s) + 128) / 256) as i32
}

/// Ruolo di un testo di sistema: sceglie il font di default (vedi `fonts/README.md`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Ui,
    Mono,
    Reading,
}

/// Font di default per ruolo, percorso di riferimento nel repo. I byte sono incorporati in `fonts.rs`.
pub const fn default_font(r: Role) -> &'static str {
    match r {
        Role::Ui => "fonts/Inter/Inter-Variable.ttf",
        Role::Mono => "fonts/JetBrainsMono/JetBrainsMono-Regular.ttf",
        Role::Reading => "fonts/NotoSerif/NotoSerif-Regular.ttf",
    }
}
