//! Render 3D a prospettiva su buffer ARGB u32, senza float. Camera dietro al kart `cam`.
//! Pista piatta (z=0) disegnata per prima, poi oggetti (kart, scatole, gusci, banane) ordinati
//! per profondità (painter's), poi HUD con glifi 3x5. Piano vicino: i poligoni vengono tagliati.

use crate::{
    fx::{dir, int, isqrt, mul, Fx, ONE},
    kart::Kart,
    race::{Haz, Race, N_BOXES},
    track::Track,
};

const SKY_TOP: u32 = 0xFF1565C0;
const SKY_LOW: u32 = 0xFFBBDEFB;
const GRASS: u32 = 0xFF2E7D32;
const ROAD: [u32; 2] = [0xFF616161, 0xFF6D6D6D];
const KERB: [u32; 2] = [0xFFD32F2F, 0xFFEEEEEE];
const BOX: u32 = 0xFFFFD600;
const BANANA: u32 = 0xFFFFEB3B;
const SHELL: u32 = 0xFFE53935;
const WHITE: u32 = 0xFFFFFFFF;
const SUN: u32 = 0xFFFFF176;
const SUN_GLOW: u32 = 0xFFFFF9C4;
const DUST: u32 = 0xFFB0BEC5;
const BLACK: u32 = 0xFF000000;
const COLORS: [u32; 8] = [
    0xFFFF1744, 0xFF2979FF, 0xFF00E676, 0xFFFF9100, 0xFFD500F9, 0xFF00E5FF, 0xFFFF80AB, 0xFFEEEEEE,
];

/// Distanza della camera dietro al kart, altezza, piano vicino.
const BACK: Fx = int(60);
const UP: Fx = int(30);
const NEAR: Fx = int(4);
/// Mezza lunghezza e mezza larghezza del kart, altezza.
const KART_HL: Fx = int(10);
const KART_HW: Fx = int(6);
const KART_H: Fx = int(8);
const KERB_W: i32 = 4;
const SUBS: i32 = 8;
const N_TRACK: usize = 640;
/// Distanza (16.16) oltre la quale la nebbia è al massimo.
const FOG_FAR: Fx = int(900);
const N_OBJ: usize = 256;

#[derive(Clone, Copy, Default)]
struct V3 {
    x: Fx,
    y: Fx,
    z: Fx,
}

#[derive(Clone, Copy, Default)]
struct Quad {
    p: [(i32, i32); 6],
    n: usize,
    key: Fx,
    col: u32,
}

/// Camera: origine, base orizzontale (`f` avanti, `r` destra), focale in px.
struct Cam {
    e: V3,
    f: (Fx, Fx),
    r: (Fx, Fx),
    w: i32,
    h: i32,
    focal: i64,
}

impl Cam {
    fn depth(&self, v: V3) -> Fx {
        mul(v.x - self.e.x, self.f.0) + mul(v.y - self.e.y, self.f.1)
    }

    /// Proiezione di un vertice con profondità `d` (>= NEAR). L'orizzonte sta a 2/5 dell'altezza.
    fn project(&self, v: V3, d: Fx) -> (i32, i32) {
        let (dx, dy) = (v.x - self.e.x, v.y - self.e.y);
        let r = mul(dx, self.r.0) + mul(dy, self.r.1);
        let sx = self.w / 2 + (r as i64 * self.focal / d as i64) as i32;
        let sy = self.h * 2 / 5 - ((v.z - self.e.z) as i64 * self.focal / d as i64) as i32;
        (sx, sy)
    }

    /// Taglia il poligono al piano vicino, lo proietta e lo accoda a `list`.
    fn add(&self, poly: &[V3], col: u32, list: &mut [Quad], len: &mut usize) {
        let mut c = [V3::default(); 6];
        let mut k = 0;
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            let (da, db) = (self.depth(a), self.depth(b));
            if da >= NEAR {
                c[k] = a;
                k += 1;
            }
            if (da >= NEAR) != (db >= NEAR) {
                let t = crate::fx::div(NEAR - da, db - da);
                c[k] = V3 {
                    x: a.x + mul(b.x - a.x, t),
                    y: a.y + mul(b.y - a.y, t),
                    z: a.z + mul(b.z - a.z, t),
                };
                k += 1;
            }
        }
        if k < 3 || *len >= list.len() {
            return;
        }
        let mut q = Quad { n: k, col, ..Default::default() };
        let mut sum = 0;
        for i in 0..k {
            let d = self.depth(c[i]).max(NEAR);
            q.p[i] = self.project(c[i], d);
            sum += d;
        }
        q.key = sum / k as Fx;
        list[*len] = q;
        *len += 1;
    }

    /// Faccia di un solido visibile se il suo centro sta dal lato della camera (back-face).
    fn visible(&self, fc: V3, bc: V3) -> bool {
        let a = [(self.e.x - fc.x) as i64, (self.e.y - fc.y) as i64, (self.e.z - fc.z) as i64];
        let b = [(fc.x - bc.x) as i64, (fc.y - bc.y) as i64, (fc.z - bc.z) as i64];
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2] > 0
    }
}

/// Scurisce un colore: `k` in 0..=256.
fn shade(c: u32, k: u32) -> u32 {
    let s = |sh: u32| (((c >> sh) & 255) * k / 256) << sh;
    0xFF000000 | s(16) | s(8) | s(0)
}

/// Sfumatura tra due colori, `t` in 0..=256.
fn lerp(a: u32, b: u32, t: u32) -> u32 {
    let ch = |sh: u32| {
        let (x, y) = ((a >> sh) & 255, (b >> sh) & 255);
        ((x * (256 - t) + y * t) / 256) << sh
    };
    0xFF000000 | ch(16) | ch(8) | ch(0)
}

fn fill(buf: &mut [u32], w: usize, h: usize, q: &Quad) {
    let p = &q.p[..q.n];
    let y0 = p.iter().map(|v| v.1).min().unwrap_or(0).max(0);
    let y1 = p.iter().map(|v| v.1).max().unwrap_or(0).min(h as i32 - 1);
    for y in y0..=y1 {
        let (mut lo, mut hi) = (i32::MAX, i32::MIN);
        for i in 0..q.n {
            let (a, b) = (p[i], p[(i + 1) % q.n]);
            if (a.1 <= y && b.1 > y) || (b.1 <= y && a.1 > y) {
                let x = a.0 + ((y - a.1) as i64 * (b.0 - a.0) as i64 / (b.1 - a.1) as i64) as i32;
                lo = lo.min(x);
                hi = hi.max(x);
            }
        }
        if lo > hi {
            continue;
        }
        let (x0, x1) = (lo.max(0), (hi + 1).min(w as i32));
        let row = y as usize * w;
        for x in x0..x1 {
            buf[row + x as usize] = q.col;
        }
    }
}

/// Ordina dalla più lontana alla più vicina (insertion sort) e disegna.
fn paint(buf: &mut [u32], w: usize, h: usize, list: &mut [Quad]) {
    for i in 1..list.len() {
        let mut j = i;
        while j > 0 && list[j - 1].key < list[j].key {
            list.swap(j - 1, j);
            j -= 1;
        }
    }
    for q in list.iter() {
        // Nebbia di profondità: i poligoni lontani tendono al colore del cielo (max metà).
        let t = (q.key.max(0) as i64 * 128 / FOG_FAR as i64).min(128) as u32;
        let mut f = *q;
        f.col = lerp(q.col, SKY_LOW, t);
        fill(buf, w, h, &f);
    }
}

/// Miscela `s` su `d` con opacità `a` (0..=256).
fn blend(d: u32, s: u32, a: u32) -> u32 {
    let ch = |sh: u32| {
        let (x, y) = ((d >> sh) & 255, (s >> sh) & 255);
        ((x * (256 - a) + y * a) / 256) << sh
    };
    0xFF000000 | ch(16) | ch(8) | ch(0)
}

/// Sole con alone, sopra l'orizzonte.
fn sun(buf: &mut [u32], w: usize, h: usize, hor: usize) {
    let r = (h / 12) as i32;
    let (cx, cy) = (w as i32 * 3 / 4, hor as i32 - r * 3 / 2);
    for dy in -3 * r..=3 * r {
        let y = cy + dy;
        if y < 0 || y >= hor as i32 {
            continue;
        }
        for dx in -3 * r..=3 * r {
            let x = cx + dx;
            if x < 0 || x >= w as i32 {
                continue;
            }
            let d2 = dx * dx + dy * dy;
            let i = y as usize * w + x as usize;
            if d2 <= r * r {
                buf[i] = blend(buf[i], SUN, 256);
            } else if d2 <= 4 * r * r {
                buf[i] = blend(buf[i], SUN_GLOW, 60);
            } else if d2 <= 9 * r * r {
                buf[i] = blend(buf[i], SUN_GLOW, 24);
            }
        }
    }
}

/// Scia di polvere: quattro quadrati a terra dietro al kart, più grandi più lontani.
fn dust(cam: &Cam, k: &Kart, tick: u32, list: &mut [Quad], len: &mut usize) {
    if k.v < 2 * ONE {
        return;
    }
    let f = dir(k.h);
    let r = (f.1, -f.0);
    for i in 1..=4 {
        let back = int(9 * i);
        let sway = int(((tick as i32 + 3 * i) % 5) - 2);
        let cx = k.x - mul(f.0, back) + mul(r.0, sway);
        let cy = k.y - mul(f.1, back) + mul(r.1, sway);
        let s = int(i + 1);
        let c = |sx: Fx, sy: Fx| V3 { x: cx + mul(r.0, sx) - mul(f.0, sy), y: cy + mul(r.1, sx) - mul(f.1, sy), z: int(1) };
        let poly = [c(-s, -s), c(s, -s), c(s, s), c(-s, s)];
        cam.add(&poly, lerp(DUST, GRASS, 40 * i as u32), list, len);
    }
}

/// Strisce di strada e cordoli, un quadrilatero per sottosegmento.
fn track_quads(cam: &Cam, tr: &Track, list: &mut [Quad], len: &mut usize) {
    let n = tr.n();
    let hw = tr.half_w;
    for seg in 0..n {
        let (a, b) = (tr.pts[seg], tr.pts[(seg + 1) % n]);
        let (dx, dy) = ((b.0 - a.0) as i64, (b.1 - a.1) as i64);
        let l = isqrt((dx * dx + dy * dy) as u64).max(1) as i64;
        // Perpendicolare alla pista, lunga `len` unità, in 16.16.
        let perp = |len: i32| {
            (((-dy) * len as i64 * ONE as i64 / l) as Fx, (dx * len as i64 * ONE as i64 / l) as Fx)
        };
        let (nx, ny) = perp(hw);
        let (kx, ky) = perp(hw + KERB_W);
        for s in 0..SUBS {
            let (ax, ay) = tr.point(seg, s * ONE / SUBS);
            let (bx, by) = tr.point(seg, (s + 1) * ONE / SUBS);
            let g = |x: Fx, y: Fx, ox: Fx, oy: Fx| V3 { x: x + ox, y: y + oy, z: 0 };
            let l0 = g(ax, ay, nx, ny);
            let r0 = g(ax, ay, -nx, -ny);
            let l1 = g(bx, by, nx, ny);
            let r1 = g(bx, by, -nx, -ny);
            let kl0 = g(ax, ay, kx, ky);
            let kl1 = g(bx, by, kx, ky);
            let kr0 = g(ax, ay, -kx, -ky);
            let kr1 = g(bx, by, -kx, -ky);
            let t = ((seg as i32 * SUBS + s) & 1) as usize;
            cam.add(&[r0, l0, l1, r1], ROAD[t], list, len);
            cam.add(&[kl0, l0, l1, kl1], KERB[t], list, len);
            cam.add(&[kr0, r0, r1, kr1], KERB[1 - t], list, len);
        }
    }
}

/// Solido a scatola con base `zb`, asse avanti (fx,fy), mezze dimensioni `hl`,`hw` e altezza `hh`.
#[allow(clippy::too_many_arguments)]
fn solid(cam: &Cam, c: (Fx, Fx), zb: Fx, f: (Fx, Fx), dims: (Fx, Fx, Fx), col: u32, list: &mut [Quad], len: &mut usize) {
    let (cx, cy) = c;
    let (hl, hw, hh) = dims;
    let r = (f.1, -f.0);
    let v = |sf: i32, sr: i32, sz: i32| V3 {
        x: cx + mul(f.0, hl * sf) + mul(r.0, hw * sr),
        y: cy + mul(f.1, hl * sf) + mul(r.1, hw * sr),
        z: zb + hh * sz,
    };
    let bc = V3 { x: cx, y: cy, z: zb + hh / 2 };
    let faces: [([V3; 4], u32); 5] = [
        ([v(-1, -1, 1), v(1, -1, 1), v(1, 1, 1), v(-1, 1, 1)], col),
        ([v(1, -1, 0), v(1, 1, 0), v(1, 1, 1), v(1, -1, 1)], shade(col, 200)),
        ([v(-1, 1, 0), v(-1, -1, 0), v(-1, -1, 1), v(-1, 1, 1)], shade(col, 150)),
        ([v(-1, -1, 0), v(1, -1, 0), v(1, -1, 1), v(-1, -1, 1)], shade(col, 180)),
        ([v(1, 1, 0), v(-1, 1, 0), v(-1, 1, 1), v(1, 1, 1)], shade(col, 180)),
    ];
    for (poly, c) in faces.iter() {
        let s = poly.iter().fold(V3::default(), |a, p| V3 { x: a.x + p.x / 4, y: a.y + p.y / 4, z: a.z + p.z / 4 });
        if cam.visible(s, bc) {
            cam.add(poly, *c, list, len);
        }
    }
}

/// Alberi a due sottosegmenti di distanza ai lati della pista: tronco e chioma a scatola.
fn trees(cam: &Cam, tr: &Track, list: &mut [Quad], len: &mut usize) {
    let n = tr.n();
    let off = tr.half_w + 34;
    for seg in (0..n).step_by(3) {
        let (ax, ay) = tr.point(seg, 0);
        let (bx, by) = tr.point(seg, ONE);
        let (dx, dy) = ((bx - ax) as i64, (by - ay) as i64);
        let l = isqrt((dx * dx + dy * dy) as u64).max(1) as i64;
        let nx = ((-dy) * off as i64 * ONE as i64 / l) as Fx;
        let ny = (dx * off as i64 * ONE as i64 / l) as Fx;
        let (cx, cy) = tr.point(seg, ONE / 2);
        for side in [-1, 1] {
            let (x, y) = (cx + side * nx, cy + side * ny);
            let f = (ONE, 0);
            // Pino a due blocchi: tronco, chioma inferiore larga, chioma superiore più stretta.
            solid(cam, (x, y), 0, f, (int(1), int(1), int(8)), 0xFF6B4423, list, len);
            solid(cam, (x, y), int(8), f, (int(4), int(4), int(5)), 0xFF2E7D32, list, len);
            solid(cam, (x, y), int(13), f, (int(3), int(3), int(5)), 0xFF43A047, list, len);
        }
    }
}

pub fn draw(race: &Race, buf: &mut [u32], w: usize, h: usize, cam: usize) {
    let hor = h * 2 / 5;
    for y in 0..h {
        let c = if y < hor { lerp(SKY_TOP, SKY_LOW, (y * 256 / hor.max(1)) as u32) } else { GRASS };
        buf[y * w..(y + 1) * w].fill(c);
    }
    let k = race.karts[cam];
    let f = dir(k.h);
    let cm = Cam {
        e: V3 { x: k.x - mul(f.0, BACK), y: k.y - mul(f.1, BACK), z: UP },
        f,
        r: (f.1, -f.0),
        w: w as i32,
        h: h as i32,
        focal: h as i64 * 2 / 3,
    };

    sun(buf, w, h, hor);
    let mut tq = [Quad::default(); N_TRACK];
    let mut nt = 0;
    track_quads(&cm, race.track, &mut tq, &mut nt);
    trees(&cm, race.track, &mut tq, &mut nt);
    paint(buf, w, h, &mut tq[..nt]);

    let mut oq = [Quad::default(); N_OBJ];
    let mut no = 0;
    let n = race.track.n();
    for i in 0..race.n {
        dust(&cm, &race.karts[i], race.tick, &mut oq, &mut no);
    }
    for b in 0..N_BOXES {
        if race.boxes[b] == 0 {
            let (x, y) = race.track.point(b * n / N_BOXES, ONE / 2);
            solid(&cm, (x, y), int(3), (ONE, 0), (int(3), int(3), int(6)), BOX, &mut oq, &mut no);
        }
    }
    for hz in race.haz.iter().filter(|z| z.kind != Haz::None) {
        let col = if hz.kind == Haz::Banana { BANANA } else { SHELL };
        solid(&cm, (hz.x, hz.y), 0, (ONE, 0), (int(2), int(2), int(3)), col, &mut oq, &mut no);
    }
    for i in 0..race.n {
        let kk = race.karts[i];
        let fk = dir(kk.h);
        solid(&cm, (kk.x, kk.y), 0, fk, (KART_HL, KART_HW, KART_H), COLORS[i], &mut oq, &mut no);
        // Abitacolo con vetro, quattro ruote scure agli angoli.
        solid(&cm, (kk.x, kk.y), KART_H, fk, (KART_HL / 2, KART_HW / 2, int(4)), 0xFF90CAF9, &mut oq, &mut no);
        let r = (fk.1, -fk.0);
        for (sf, sr) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
            let wx = kk.x + mul(fk.0, KART_HL * sf / 2 + sf * int(2)) + mul(r.0, KART_HW * sr);
            let wy = kk.y + mul(fk.1, KART_HL * sf / 2 + sf * int(2)) + mul(r.1, KART_HW * sr);
            solid(&cm, (wx, wy), 0, fk, (int(3), int(1), int(3)), 0xFF212121, &mut oq, &mut no);
        }
    }
    paint(buf, w, h, &mut oq[..no]);

    hud(race, buf, w, h, cam);
}

fn rect(buf: &mut [u32], w: usize, h: usize, x: i32, y: i32, rw: i32, rh: i32, c: u32) {
    for py in y.max(0)..(y + rh).min(h as i32) {
        for px in x.max(0)..(x + rw).min(w as i32) {
            buf[py as usize * w + px as usize] = c;
        }
    }
}

/// Glifo 3x5: ogni riga è una maschera a 3 bit (bit 2 = colonna sinistra).
fn glyph(c: u8) -> [u8; 5] {
    match c {
        b'0' => [2, 5, 5, 5, 2],
        b'1' => [2, 6, 2, 2, 7],
        b'2' => [7, 1, 7, 4, 7],
        b'3' => [7, 1, 7, 1, 7],
        b'4' => [5, 5, 7, 1, 1],
        b'5' => [7, 4, 7, 1, 7],
        b'6' => [7, 4, 7, 5, 7],
        b'7' => [7, 1, 1, 1, 1],
        b'8' => [7, 5, 7, 5, 7],
        b'9' => [7, 5, 7, 1, 7],
        b'L' => [4, 4, 4, 4, 7],
        b'A' => [2, 5, 7, 5, 5],
        b'P' => [6, 5, 6, 4, 4],
        b'O' => [7, 5, 5, 5, 7],
        b'S' => [6, 4, 2, 1, 6],
        b'/' => [1, 1, 2, 4, 4],
        _ => [0; 5],
    }
}

fn text(buf: &mut [u32], w: usize, h: usize, x0: i32, y0: i32, s: i32, t: &[u8], col: u32) {
    for (i, &c) in t.iter().enumerate() {
        for (ry, bits) in glyph(c).iter().enumerate() {
            for cx in 0..3usize {
                if (bits >> (2 - cx)) & 1 == 1 {
                    let x = x0 + (i as i32 * 4 + cx as i32) * s;
                    rect(buf, w, h, x, y0 + ry as i32 * s, s, s, col);
                }
            }
        }
    }
}

/// HUD: giro in alto a sinistra, posizione in alto a destra.
fn hud(race: &Race, buf: &mut [u32], w: usize, h: usize, cam: usize) {
    let s = (h / 120).max(1) as i32;
    let laps = race.track.laps;
    let lap = race.karts[cam].count.saturating_add(1).min(laps);
    let pos = race.order[..race.n].iter().position(|&i| i as usize == cam).unwrap_or(0) + 1;
    let a = [b'L', b'A', b'P', b' ', b'0' + lap, b'/', b'0' + laps];
    let b = [b'P', b'O', b'S', b' ', b'0' + pos as u8, b'/', b'0' + race.n as u8];
    let pad = 2 * s;
    let wb = b.len() as i32 * 4 * s;
    text(buf, w, h, pad + s, pad + s, s, &a, BLACK);
    text(buf, w, h, pad, pad, s, &a, WHITE);
    let x = w as i32 - pad - wb;
    text(buf, w, h, x + s, pad + s, s, &b, BLACK);
    text(buf, w, h, x, pad, s, &b, WHITE);
}
