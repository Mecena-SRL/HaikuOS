//! Host vettoriale (solo std). Ogni elemento è una forma analitica (segmento, cerchio,
//! rettangolo, triangolo) valutata con la distanza con bordo antialias: nessuna texture,
//! nessun bitmap, nitido a qualsiasi risoluzione (1920×1080, 800×480, ...).
//! Coordinate del tavolo: unità = px logici del core.

use pinball::{Pinball, State, BUMPERS, BUMPER_R, FP, KICKERS, SENSORS, TABLE_H, TABLE_W, WALLS};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Title,
    Play,
    Pause,
    Over,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Next {
    Go(Screen),
    Quit,
    Stay,
}

/// Titolo: invio avvia. Pausa: `r` o invio riprende. Fine: `r` o invio ricomincia. `q` esce.
pub fn on_menu_key(s: Screen, key: &str) -> Next {
    match (s, key) {
        (Screen::Play, _) => Next::Stay,
        (_, "q") => Next::Quit,
        (Screen::Title, _) => Next::Go(Screen::Play),
        (Screen::Pause | Screen::Over, "r" | "") => Next::Go(Screen::Play),
        _ => Next::Stay,
    }
}

type P = (f32, f32);
type Rgb = [f32; 3];

#[derive(Clone, Copy)]
enum Shape {
    Seg(P, P, f32),
    Circ(P, f32),
    /// centro, semilati, raggio d'angolo
    Rect(P, P, f32),
    /// convesso, antiorario (vedi `tri`)
    Tri([P; 3]),
}

#[derive(Clone, Copy)]
struct Prim {
    s: Shape,
    col: Rgb,
    a: f32,
}

fn p(s: Shape, col: Rgb) -> Prim {
    Prim { s, col, a: 1.0 }
}

fn tri(mut v: [P; 3]) -> Shape {
    let area = (v[1].0 - v[0].0) * (v[2].1 - v[0].1) - (v[1].1 - v[0].1) * (v[2].0 - v[0].0);
    if area < 0.0 {
        v.swap(1, 2);
    }
    Shape::Tri(v)
}

fn sub(a: P, b: P) -> P {
    (a.0 - b.0, a.1 - b.1)
}

fn dot(a: P, b: P) -> f32 {
    a.0 * b.0 + a.1 * b.1
}

/// Distanza con segno (negativa dentro), in unità del tavolo.
fn sdf(s: &Shape, q: P) -> f32 {
    match *s {
        Shape::Seg(a, b, r) => {
            let (pa, ba) = (sub(q, a), sub(b, a));
            let h = (dot(pa, ba) / dot(ba, ba)).clamp(0.0, 1.0);
            let d = sub(pa, (ba.0 * h, ba.1 * h));
            dot(d, d).sqrt() - r
        }
        Shape::Circ(c, r) => {
            let d = sub(q, c);
            dot(d, d).sqrt() - r
        }
        Shape::Rect(c, h, r) => {
            let d = sub(q, c);
            let (qx, qy) = (d.0.abs() - h.0 + r, d.1.abs() - h.1 + r);
            let (ox, oy) = (qx.max(0.0), qy.max(0.0));
            (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r
        }
        Shape::Tri(v) => {
            // massimo delle distanze dai lati uscenti: esatta dentro, approssimata fuori
            let mut d = f32::MIN;
            for i in 0..3 {
                let (a, b) = (v[i], v[(i + 1) % 3]);
                let e = sub(b, a);
                let l = dot(e, e).sqrt();
                d = d.max(dot(sub(q, a), (e.1 / l, -e.0 / l)));
            }
            d
        }
    }
}

fn bbox(s: &Shape) -> (P, P) {
    match *s {
        Shape::Seg(a, b, r) => ((a.0.min(b.0) - r, a.1.min(b.1) - r), (a.0.max(b.0) + r, a.1.max(b.1) + r)),
        Shape::Circ(c, r) => ((c.0 - r, c.1 - r), (c.0 + r, c.1 + r)),
        Shape::Rect(c, h, r) => ((c.0 - h.0 - r, c.1 - h.1 - r), (c.0 + h.0 + r, c.1 + h.1 + r)),
        Shape::Tri(v) => (
            (v[0].0.min(v[1].0).min(v[2].0), v[0].1.min(v[1].1).min(v[2].1)),
            (v[0].0.max(v[1].0).max(v[2].0), v[0].1.max(v[1].1).max(v[2].1)),
        ),
    }
}

/// Fotogramma RGB.
pub struct Frame {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[u8; 3]>,
}

impl Frame {
    pub fn write_ppm(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        write!(out, "P6\n{} {}\n255\n", self.w, self.h)?;
        for c in &self.px {
            out.write_all(c)?;
        }
        Ok(())
    }
}

/// Mappa tavolo→pixel: scala `s`, offset `ox`, `oy`.
#[derive(Clone, Copy)]
struct Map {
    s: f32,
    ox: f32,
    oy: f32,
}

/// Disegna le forme nel buffer; ognuna tocca solo il suo riquadro.
fn paint(buf: &mut [Rgb], w: usize, h: usize, m: Map, prims: &[Prim]) {
    for pr in prims {
        let (lo, hi) = bbox(&pr.s);
        let x0 = ((m.ox + lo.0 * m.s).floor() as isize - 1).clamp(0, w as isize - 1) as usize;
        let x1 = ((m.ox + hi.0 * m.s).ceil() as isize + 1).clamp(0, w as isize - 1) as usize;
        let y0 = ((m.oy + lo.1 * m.s).floor() as isize - 1).clamp(0, h as isize - 1) as usize;
        let y1 = ((m.oy + hi.1 * m.s).ceil() as isize + 1).clamp(0, h as isize - 1) as usize;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let q = (((x as f32 + 0.5) - m.ox) / m.s, ((y as f32 + 0.5) - m.oy) / m.s);
                // cerchi: scarta gli angoli del riquadro senza radice quadrata
                if let Shape::Circ(c, r) = pr.s {
                    let d = sub(q, c);
                    if dot(d, d) > (r + 1.0) * (r + 1.0) {
                        continue;
                    }
                }
                let cov = (0.5 - sdf(&pr.s, q) * m.s).clamp(0.0, 1.0) * pr.a;
                if cov > 0.0 {
                    let c = &mut buf[y * w + x];
                    for k in 0..3 {
                        c[k] += (pr.col[k] - c[k]) * cov;
                    }
                }
            }
        }
    }
}

const WALL: Rgb = [0.36, 0.44, 0.66];
const KICK: Rgb = [0.45, 0.95, 1.0];
const FLIP_REST: Rgb = [0.80, 0.42, 0.14];
const FLIP_UP: Rgb = [1.0, 0.86, 0.38];
const BUMP: Rgb = [0.95, 0.22, 0.56];
const LIT: Rgb = [0.35, 1.0, 0.55];
const UNLIT: Rgb = [0.20, 0.30, 0.46];
const BALL: Rgb = [0.92, 0.95, 1.0];
const HUD: Rgb = [1.0, 0.95, 0.62];
const DARK: Rgb = [0.03, 0.04, 0.10];

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Segmenti del display a sette segmenti (a b c d e f g = bit 0..6), per le cifre 0..9.
const DIGIT: [u8; 10] = [63, 6, 91, 79, 102, 109, 125, 7, 127, 111];

/// Numero a sette segmenti vettoriali. `hgt` in unità del tavolo; ritorna la larghezza.
fn digits(out: &mut Vec<Prim>, n: u32, x: f32, y: f32, hgt: f32, col: Rgb) -> f32 {
    let (w, m, t) = (hgt * 0.6, hgt * 0.5, hgt * 0.09);
    let gap = hgt * 0.35;
    let mut cx = x;
    for ch in n.to_string().bytes() {
        let mask = DIGIT[(ch - b'0') as usize];
        let at = |a: P| (cx + a.0, y + a.1);
        let segs: [(P, P); 7] = [
            ((0.0, 0.0), (w, 0.0)),
            ((w, 0.0), (w, m)),
            ((w, m), (w, hgt)),
            ((0.0, hgt), (w, hgt)),
            ((0.0, m), (0.0, hgt)),
            ((0.0, 0.0), (0.0, m)),
            ((0.0, m), (w, m)),
        ];
        for (k, (a, b)) in segs.iter().enumerate() {
            if mask & (1 << k) != 0 {
                out.push(p(Shape::Seg(at(*a), at(*b), t), col));
            }
        }
        cx += w + gap;
    }
    cx - x - gap
}

fn digits_width(n: u32, hgt: f32) -> f32 {
    let k = n.to_string().len() as f32;
    k * hgt * 0.6 + (k - 1.0) * hgt * 0.35
}

/// Tavolo: sempre disegnato.
fn table(g: &Pinball, out: &mut Vec<Prim>) {
    let f = |v: f32| v / FP as f32;
    let fp = |v: (i64, i64)| (f(v.0 as f32), f(v.1 as f32));
    // sfondo a tema: aloni morbidi e stelle, dietro al tavolo
    for (c, r, col, a) in [
        ((80.0, 60.0), 150.0, [0.10, 0.25, 0.45], 0.35),
        ((120.0, 200.0), 120.0, [0.35, 0.08, 0.30], 0.30),
    ] {
        out.push(Prim { s: Shape::Circ(c, r), col, a });
    }
    for k in 0..24u32 {
        let x = -60.0 + (k * 37 % 280) as f32;
        let y = -50.0 + (k * 53 % 340) as f32;
        out.push(Prim { s: Shape::Circ((x, y), 0.6 + (k % 3) as f32 * 0.3), col: [0.85, 0.9, 1.0], a: 0.5 });
    }
    // faro e onde del porto, fuori dal tavolo
    out.push(Prim { s: Shape::Tri([(-30.0, 230.0), (-18.0, 230.0), (-22.0, 120.0)]), col: [0.8, 0.9, 0.6], a: 0.10 });
    out.push(p(Shape::Tri([(-27.0, 150.0), (-17.0, 150.0), (-19.0, 128.0)]), [0.25, 0.28, 0.38]));
    out.push(p(Shape::Rect((-22.0, 170.0), (4.0, 46.0), 1.0), [0.25, 0.28, 0.38]));
    out.push(Prim { s: Shape::Circ((-22.0, 124.0), 4.0), col: [1.0, 0.9, 0.5], a: 0.25 });
    out.push(p(Shape::Circ((-22.0, 124.0), 2.2), [1.0, 0.92, 0.6]));
    for k in 0..3u32 {
        let y = 222.0 + k as f32 * 6.0;
        out.push(Prim { s: Shape::Rect((80.0, y), (140.0, 1.2), 1.0), col: [0.20, 0.55, 0.65], a: 0.18 });
    }
    out.push(p(Shape::Rect((80.0, 120.0), (80.0, 120.0), 4.0), DARK));
    for w in WALLS {
        out.push(p(Shape::Seg((w[0] as f32, w[1] as f32), (w[2] as f32, w[3] as f32), 0.9), WALL));
    }
    for k in KICKERS {
        out.push(p(Shape::Seg((k[0] as f32, k[1] as f32), (k[2] as f32, k[3] as f32), 2.0), KICK));
    }
    for c in BUMPERS {
        let ctr = (c.0 as f32, c.1 as f32);
        let r = f(BUMPER_R as f32);
        out.push(Prim { s: Shape::Circ(ctr, r + 7.0), col: BUMP, a: 0.07 });
        out.push(Prim { s: Shape::Circ(ctr, r + 3.0), col: BUMP, a: 0.18 });
        out.push(p(Shape::Circ(ctr, r), BUMP));
        out.push(p(Shape::Circ(ctr, r - 2.4), DARK));
        out.push(p(Shape::Circ(ctr, 2.0), [1.0, 0.8, 0.9]));
    }
    for (j, s) in SENSORS.iter().enumerate() {
        let lit = g.lit & (1u8 << j) != 0;
        let ctr = (s.0 as f32, s.1 as f32);
        out.push(p(Shape::Circ(ctr, 3.0), if lit { LIT } else { UNLIT }));
        out.push(p(Shape::Circ(ctr, 1.4), DARK));
    }
    for fl in &g.flips {
        let (a, b) = (fp(fl.pivot), fp(fl.tip()));
        let up = (fl.dps.abs() as f32 / 20.0).min(1.0);
        out.push(Prim { s: Shape::Seg(a, b, 5.0), col: FLIP_UP, a: 0.12 + 0.12 * up });
        out.push(p(Shape::Seg(a, b, 2.8), mix(FLIP_REST, FLIP_UP, up)));
        out.push(p(Shape::Circ(a, 3.6), [0.2, 0.12, 0.05]));
        out.push(p(Shape::Circ(a, 1.4), FLIP_UP));
    }
    for b in g.balls.iter().filter(|b| b.live) {
        let c = fp(b.pos);
        let v = fp(b.vel);
        let sp = (v.0 * v.0 + v.1 * v.1).sqrt().max(1e-3);
        let u = (v.0 / sp, v.1 / sp);
        for k in 1..=4u32 {
            let kk = k as f32;
            let at = (c.0 - u.0 * kk * 1.6, c.1 - u.1 * kk * 1.6);
            out.push(Prim { s: Shape::Circ(at, 2.6 - kk * 0.4), col: [0.7, 0.9, 1.0], a: 0.30 - kk * 0.06 });
        }
        out.push(Prim { s: Shape::Circ(c, 4.2), col: [0.6, 0.8, 1.0], a: 0.15 });
        out.push(p(Shape::Circ(c, 3.0), BALL));
        out.push(p(Shape::Circ((c.0 - 0.9, c.1 - 0.9), 1.0), [1.0, 1.0, 1.0]));
    }
}

/// HUD in gioco: punteggio e vite in alto nel tavolo.
fn hud(g: &Pinball, out: &mut Vec<Prim>) {
    digits(out, g.score, 6.0, 5.0, 10.0, HUD);
    // moltiplicatore: quattro pallini, accesi fino al livello attuale
    for i in 0..4u32 {
        let on = i + 1 < g.mult;
        out.push(p(Shape::Circ((6.0 + i as f32 * 6.0, 20.0), 2.0), if on { LIT } else { UNLIT }));
    }
    // carica della leva nella corsia, solo in attesa di lancio
    if g.state == State::Ready {
        let h = g.charge as f32 * 1.8;
        out.push(p(Shape::Rect((149.0, 232.0 - h / 2.0), (1.2, h / 2.0), 1.0), mix(LIT, HUD, g.charge as f32 / 100.0)));
    }
    for i in 0..3u8 {
        let on = i < g.lives;
        out.push(p(Shape::Circ((150.0 - i as f32 * 8.0, 10.0), 2.5), if on { BALL } else { UNLIT }));
    }
}

/// Pannello di menu centrato nel tavolo, con il contenuto della schermata.
fn overlay(g: &Pinball, s: Screen, out: &mut Vec<Prim>) {
    let c = (80.0, 120.0);
    out.push(p(Shape::Rect(c, (62.0, 48.0), 5.0), [0.9, 0.55, 0.2]));
    out.push(p(Shape::Rect(c, (60.0, 46.0), 4.0), [0.04, 0.05, 0.12]));
    match s {
        Screen::Title => {
            out.push(p(tri([(68.0, 88.0), (68.0, 112.0), (88.0, 100.0)]), FLIP_UP));
            // la guardiana Mabel: cappello, testa e corpo
            out.push(p(Shape::Rect((121.0, 113.0), (7.0, 8.0), 2.0), [0.2, 0.25, 0.4]));
            out.push(p(Shape::Circ((121.0, 100.0), 4.5), [0.95, 0.8, 0.65]));
            out.push(p(tri([(111.0, 96.0), (131.0, 96.0), (121.0, 86.0)]), [0.7, 0.2, 0.25]));
            out.push(Prim { s: Shape::Circ((121.0, 100.0), 9.0), col: [1.0, 0.85, 0.5], a: 0.10 });
        }
        Screen::Pause => {
            out.push(p(Shape::Rect((74.0, 110.0), (3.0, 11.0), 1.0), FLIP_UP));
            out.push(p(Shape::Rect((86.0, 110.0), (3.0, 11.0), 1.0), FLIP_UP));
        }
        Screen::Over => {
            let w = digits_width(g.score, 12.0);
            digits(out, g.score, c.0 - w / 2.0, 96.0, 12.0, HUD);
            let w = digits_width(g.high, 6.0);
            digits(out, g.high, c.0 - w / 2.0, 116.0, 6.0, UNLIT);
        }
        Screen::Play => {}
    }
    out.push(p(Shape::Rect((80.0, 132.0), (36.0, 1.5), 1.5), KICK));
}

/// Fotogramma `w`×`h`, qualsiasi risoluzione. Il tavolo è centrato in proporzione, con bordi neri.
pub fn render(g: &Pinball, screen: Screen, w: usize, h: usize) -> Frame {
    let s = (w as f32 / TABLE_W as f32).min(h as f32 / TABLE_H as f32);
    let m = Map {
        s,
        ox: (w as f32 - TABLE_W as f32 * s) / 2.0,
        oy: (h as f32 - TABLE_H as f32 * s) / 2.0,
    };
    let mut buf = vec![[0.0f32; 3]; w * h];
    for y in 0..h {
        let row = mix([0.02, 0.03, 0.08], [0.07, 0.09, 0.20], y as f32 / h as f32);
        buf[y * w..(y + 1) * w].fill(row);
    }
    let mut tab = Vec::with_capacity(64);
    table(g, &mut tab);
    if screen == Screen::Play {
        hud(g, &mut tab);
    }
    paint(&mut buf, w, h, m, &tab);
    if screen != Screen::Play {
        for c in buf.iter_mut() {
            *c = [c[0] * 0.35, c[1] * 0.35, c[2] * 0.35];
        }
        let mut ov = Vec::with_capacity(32);
        overlay(g, screen, &mut ov);
        paint(&mut buf, w, h, m, &ov);
    }
    let px = buf
        .iter()
        .map(|c| [(c[0] * 255.0 + 0.5) as u8, (c[1] * 255.0 + 0.5) as u8, (c[2] * 255.0 + 0.5) as u8])
        .collect();
    Frame { w, h, px }
}

/// Testo dei menu per il terminale.
pub fn menu_text(s: Screen, g: &Pinball) {
    match s {
        Screen::Title => println!("PINBALL\nINVIO per iniziare. Gioco: z sinistra, m destra, invio leva, q esce"),
        Screen::Pause => println!("PAUSA: r riprendi, q esci"),
        Screen::Over => println!("FINE: punti {}  record {}. INVIO ricomincia, q esci", g.score, g.high),
        Screen::Play => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titolo_pausa_fine_e_tasti() {
        assert_eq!(on_menu_key(Screen::Title, ""), Next::Go(Screen::Play));
        assert_eq!(on_menu_key(Screen::Title, "q"), Next::Quit);
        assert_eq!(on_menu_key(Screen::Pause, "r"), Next::Go(Screen::Play));
        assert_eq!(on_menu_key(Screen::Pause, "x"), Next::Stay);
        assert_eq!(on_menu_key(Screen::Over, ""), Next::Go(Screen::Play));
    }

    #[test]
    fn fotogramma_a_ogni_risoluzione() {
        for (w, h) in [(320, 480), (800, 480), (1920, 1080)] {
            let f = render(&Pinball::new(), Screen::Play, w, h);
            assert_eq!((f.w, f.h, f.px.len()), (w, h, w * h));
        }
    }

    #[test]
    fn fine_mostra_punteggio() {
        let mut g = Pinball::new();
        g.score = 1234;
        let a = render(&g, Screen::Over, 160, 240);
        let b = render(&Pinball::new(), Screen::Over, 160, 240);
        assert_ne!(a.px, b.px);
    }
}

#[cfg(test)]
mod bench {
    use super::*;

    /// `cargo test --release -- --ignored --nocapture bench`
    #[test]
    #[ignore]
    fn bench_frame() {
        let g = Pinball::new();
        let t = std::time::Instant::now();
        for _ in 0..50 {
            std::hint::black_box(render(&g, Screen::Play, 800, 480));
        }
        println!("BENCH pinball 800x480 {:.1} us/frame", t.elapsed().as_secs_f64() * 1e6 / 50.0);
    }
}
