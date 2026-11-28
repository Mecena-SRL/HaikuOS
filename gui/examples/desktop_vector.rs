//! Scrivania reale disegnata da `render::draw_motion` su un `Canvas` a piena risoluzione.
//! Unità logiche 240×135, scala intera (16 a 3840×2160, 8 a 1920×1080). Testo vettoriale (`vfont`)
//! con antialiasing reale. Animazioni (`anim`): apertura/chiusura finestra e selettore reattivo.
//! Uso: cargo run --release --features wide-font --example desktop_vector -- W H out.ppm [modo] [icona]
//!   modo: (assente) scrivania ferma | app=N finestra a schermo intero | open=MS | close=MS | launch=MS
//!   open=MS   calcolatrice che sale a MS millisecondi; close=MS la fa scendere (chiusura)
//!   launch=MS selettore che scorre dall'icona FILE alla CALC
//!   icona: indice dell'icona da aprire prima del frame (screenshot delle app)
//! Poi il PPM diventa PNG: `python3 -c "from PIL import Image; Image.open('out.ppm').save('out.png')"`.
#![allow(dead_code)]
use haiku_gui::anim::{Budget, Ease, Reactive, FRAME_US};
use haiku_gui::icons;
use haiku_gui::input::Key;
use haiku_gui::gfx::{self, Dirty};
use haiku_gui::layout::{icon_cell, Rect};
use haiku_gui::render::{draw, draw_motion, Canvas, Motion, STILL};
use haiku_gui::{vfont, Icon, Shell, Tok};
use std::io::Write;

const LW: i32 = 240;
const LH: i32 = 135;
/// Spostamento iniziale della finestra in apertura/chiusura, px logici.
const WIN_DY: i32 = 40;
const OPEN_MS: u32 = 250;
const CLOSE_MS: u32 = 200;
const GLIDE_MS: u32 = 200;
/// Dimensione del testo vettoriale in px logici: avanzata di 6 px per carattere, come il font di sistema.
const TEXT_EM: i32 = 8;

static ICONS: [Icon; 11] = [
    Icon { label: "FILE", app: 1, title: "FILE", w: 120, h: 80 },
    Icon { label: "TERM", app: 2, title: "TERMINALE", w: 120, h: 80 },
    Icon { label: "CALC", app: 3, title: "CALCOLATRICE", w: 80, h: 80 },
    Icon { label: "WEB", app: 4, title: "BROWSER", w: 120, h: 80 },
    Icon { label: "EDIT", app: 5, title: "EDITOR", w: 120, h: 80 },
    Icon { label: "SET", app: 6, title: "IMPOST", w: 120, h: 80 },
    Icon { label: "ORA", app: 7, title: "ORA", w: 80, h: 80 },
    Icon { label: "READ", app: 8, title: "LETTORE", w: 120, h: 80 },
    Icon { label: "IDE", app: 9, title: "IDE", w: 160, h: 120 },
    Icon { label: "DOC", app: 10, title: "DOCUMENTI", w: 120, h: 80 },
    Icon { label: "SCACCHI", app: 11, title: "SCACCHI", w: 120, h: 100 },
];

/// Canvas a 32 bit: coordinate logiche × `s` = pixel fisici.
struct Vector {
    pw: i32,
    ph: i32,
    s: i32,
    /// Buffer di disegno (back buffer).
    buf: Vec<u32>,
    /// Buffer mostrato (front buffer): riceve solo le zone sporche in `present`.
    front: Vec<u32>,
    dirty: Dirty<8>,
    /// Area fisica in cui il disegno ha effetto: restringerla limita il lavoro.
    clip: Rect,
}

impl Vector {
    fn new(pw: i32, ph: i32) -> Vector {
        let n = (pw * ph) as usize;
        Vector { pw, ph, s: pw / LW, buf: vec![0; n], front: vec![0; n], dirty: Dirty::new(), clip: Rect::new(0, 0, pw, ph) }
    }

    /// Copia le zone sporche dal back buffer al front buffer e le svuota.
    fn present(&mut self) {
        for r in self.dirty.rects() {
            if let Some(r) = gfx::clip(*r, self.pw, self.ph) {
                gfx::copy(&mut self.front, &self.buf, self.pw as usize, r);
            }
        }
        self.dirty.clear();
    }

    /// Riempie il rettangolo fisico [x0,x1)×[y0,y1), limitato all'area di disegno.
    fn span(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u32) {
        if let Some(r) = gfx::intersect(Rect::new(x0, y0, x1 - x0, y1 - y0), self.clip) {
            gfx::fill(&mut self.buf, self.pw as usize, r, c);
        }
    }

    /// Mescola il colore `c` sul pixel fisico (x, y) con copertura `a` (0..=255).
    fn mix(&mut self, x: i32, y: i32, a: u8, c: u32) {
        if !self.clip.contains(x, y) {
            return;
        }
        let p = &mut self.buf[(y * self.pw + x) as usize];
        let (old, k) = (*p, a as u32);
        let ch = |sh: u32| {
            let (bg, fg) = ((old >> sh) & 255, (c >> sh) & 255);
            (bg * (255 - k) + fg * k + 127) / 255
        };
        *p = (ch(16) << 16) | (ch(8) << 8) | ch(0);
    }
}

/// Vista in pixel fisici per `vfont`: `blend` riceve coordinate fisiche e la copertura reale.
struct Px<'a>(&'a mut Vector);

impl Canvas for Px<'_> {
    fn fill(&mut self, _: Rect, _: Tok) {}
    fn text(&mut self, _: i32, _: i32, _: &str, _: Tok) {}
    fn blend(&mut self, x: i32, y: i32, a: u8, t: Tok) {
        self.0.mix(x, y, a, t.rgb());
    }
}

impl Canvas for Vector {
    fn rounded(&self) -> bool {
        true
    }

    fn rich(&self) -> bool {
        true
    }

    fn fill(&mut self, r: Rect, t: Tok) {
        let s = self.s;
        self.span(r.x * s, r.y * s, (r.x + r.w) * s, (r.y + r.h) * s, t.rgb());
    }

    fn smooth(&self) -> bool {
        true
    }

    /// Rettangolo con angoli antialiasati: copertura dalla distanza dal bordo del cerchio d'angolo.
    fn rect_rounded(&mut self, r: Rect, radius: i32, t: Tok) {
        let s = self.s;
        let (x0, y0, x1, y1) = (r.x * s, r.y * s, (r.x + r.w) * s, (r.y + r.h) * s);
        let rr = radius * s;
        let c = t.rgb();
        let Some(cl) = gfx::intersect(Rect::new(x0, y0, x1 - x0, y1 - y0), self.clip) else { return };
        let rf = rr as f32;
        for py in cl.y..cl.y + cl.h {
            // Fuori dalle zone d'angolo la riga è piena: una `span` invece di un test per pixel.
            if rr == 0 || (py >= y0 + rr && py < y1 - rr) {
                self.span(cl.x, py, cl.x + cl.w, py + 1, c);
                continue;
            }
            for px in cl.x..cl.x + cl.w {
                let (qx, qy) = (px as f32 + 0.5, py as f32 + 0.5);
                let ex = (x0 + rr) as f32 - qx;
                let ex = if ex > 0.0 { ex } else { qx - (x1 - rr) as f32 };
                let ey = (y0 + rr) as f32 - qy;
                let ey = if ey > 0.0 { ey } else { qy - (y1 - rr) as f32 };
                if ex <= 0.0 || ey <= 0.0 {
                    self.span(px, py, px + 1, py + 1, c);
                    continue;
                }
                let cov = (0.5 - ((ex * ex + ey * ey).sqrt() - rf)).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.mix(px, py, (cov * 255.0) as u8, c);
                }
            }
        }
    }

    /// Segmento antialiasato di spessore `w` logico: copertura dalla distanza dall'asse.
    fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, w: i32, t: Tok) {
        let s = self.s as f32;
        let c = t.rgb();
        let (ax, ay) = ((x1 as f32 + 0.5) * s, (y1 as f32 + 0.5) * s);
        let (bx, by) = ((x2 as f32 + 0.5) * s, (y2 as f32 + 0.5) * s);
        let hw = w.max(1) as f32 * s / 2.0;
        let (dx, dy) = (bx - ax, by - ay);
        let len2 = (dx * dx + dy * dy).max(1e-6);
        let bb = Rect::new(
            (ax.min(bx) - hw - 1.0) as i32,
            (ay.min(by) - hw - 1.0) as i32,
            ((ax.max(bx) - ax.min(bx)) + 2.0 * hw + 3.0) as i32,
            ((ay.max(by) - ay.min(by)) + 2.0 * hw + 3.0) as i32,
        );
        let Some(cl) = gfx::intersect(bb, self.clip) else { return };
        for py in cl.y..cl.y + cl.h {
            for px in cl.x..cl.x + cl.w {
                let (qx, qy) = (px as f32 + 0.5 - ax, py as f32 + 0.5 - ay);
                let k = ((qx * dx + qy * dy) / len2).clamp(0.0, 1.0);
                let (ex, ey) = (qx - k * dx, qy - k * dy);
                let cov = (hw - (ex * ex + ey * ey).sqrt() + 0.5).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.mix(px, py, (cov * 255.0) as u8, c);
                }
            }
        }
    }

    /// Poligono antialiasato (regola pari): 4×4 campioni per pixel.
    fn poly(&mut self, pts: &[(i32, i32)], t: Tok) {
        let s = self.s as f32;
        let c = t.rgb();
        let p: Vec<(f32, f32)> = pts.iter().map(|&(x, y)| (x as f32 * s, y as f32 * s)).collect();
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for &(x, y) in &p {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        let bb = Rect::new(x0 as i32, y0 as i32, (x1 - x0) as i32 + 2, (y1 - y0) as i32 + 2);
        let Some(cl) = gfx::intersect(bb, self.clip) else { return };
        for py in cl.y..cl.y + cl.h {
            for px in cl.x..cl.x + cl.w {
                let mut hits = 0u32;
                for j in 0..4 {
                    for i in 0..4 {
                        let sx = px as f32 + (i as f32 + 0.5) / 4.0;
                        let sy = py as f32 + (j as f32 + 0.5) / 4.0;
                        let mut inside = false;
                        for k in 0..p.len() {
                            let (a, b) = (p[k], p[(k + 1) % p.len()]);
                            if (a.1 > sy) != (b.1 > sy) && sx < a.0 + (sy - a.1) * (b.0 - a.0) / (b.1 - a.1) {
                                inside = !inside;
                            }
                        }
                        hits += inside as u32;
                    }
                }
                if hits > 0 {
                    self.mix(px, py, (hits * 255 / 16) as u8, c);
                }
            }
        }
    }

    /// Icona: plate arrotondato dell'accento di app, poi i segni a griglia 32 scalati nella cella.
    fn icon(&mut self, cell: Rect, label: &str, app: u8) {
        let s = self.s;
        self.rect_rounded(cell, cell.w / 8, Tok::A(app));
        let k = cell.w * s;
        let (ox, oy) = (cell.x * s, cell.y * s);
        for &(x, y, w, h, t) in icons::art(label) {
            let (x0, x1) = (x * k / 32, ((x + w) * k / 32).max(x * k / 32 + 1));
            let (y0, y1) = (y * k / 32, ((y + h) * k / 32).max(y * k / 32 + 1));
            self.span(ox + x0, oy + y0, ox + x1, oy + y1, t.rgb());
        }
    }

    /// Testo: `vfont` con antialiasing, in pixel fisici. Il font di sistema resta quello del GBA.
    fn text(&mut self, x: i32, y: i32, text: &str, t: Tok) {
        self.vtext(x, y, TEXT_EM, text, t);
    }

    fn vtext(&mut self, x: i32, y: i32, size: i32, text: &str, t: Tok) -> i32 {
        let k = self.s;
        vfont::draw_text(&mut Px(self), x * k, y * k, size * k, text, t) / k
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let w: i32 = a.get(1).and_then(|v| v.parse().ok()).unwrap_or(3840);
    let h: i32 = a.get(2).and_then(|v| v.parse().ok()).unwrap_or(2160);
    let out = a.get(3).map(String::as_str).unwrap_or("desktop.ppm");
    if w % LW != 0 || h != w / LW * LH {
        eprintln!("risoluzione {w}x{h} non multipla intera di {LW}x{LH}");
        std::process::exit(1);
    }

    let mut shell = Shell::new(&ICONS, LW, LH);
    shell.set_clock(10, 30);
    // Quinto argomento (opzionale): indice dell'icona da aprire, per gli screenshot delle app.
    if let Some(i) = a.get(5).and_then(|p| p.parse::<usize>().ok()) {
        shell.sel = i;
        let _ = shell.key(Key::A);
    }
    let mut v = Vector::new(w, h);
    // "app=N": una sola finestra a schermo intero con l'app N (es. app=3 calcolatrice).
    if let Some(n) = a.get(4).and_then(|p| p.strip_prefix("app=")).and_then(|p| p.parse::<u8>().ok()) {
        let title = ICONS.iter().find(|i| i.app == n).map(|i| i.title).unwrap_or("");
        shell.wm.open(n, title, LW - 2, LH - 2, LW, LH);
        draw(&mut v, &shell);
        let mut f = std::io::BufWriter::new(std::fs::File::create(out).expect("file"));
        write!(f, "P6\n{w} {h}\n255\n").unwrap();
        for px in &v.buf {
            f.write_all(&[(px >> 16) as u8, (px >> 8) as u8, *px as u8]).unwrap();
        }
        println!("app {n} -> {out}");
        return;
    }

    // Movimento: "open=MS", "close=MS" o "launch=MS", a MS millisecondi dall'inizio.
    let arg = a.get(4).map(String::as_str).unwrap_or("");
    let (mode, ms) = match arg.split_once('=') {
        Some((m, t)) => (m, t.parse::<u32>().ok()),
        None => ("", None),
    };
    let mut dy = [0i32];
    let mut sel = None;
    if let Some(ms) = ms {
        match mode {
            "close" => {
                shell.wm.open(3, "CALCOLATRICE", 80, 60, LW, LH);
                let mut r = Reactive::new(0);
                r.set(WIN_DY, 0, CLOSE_MS, Ease::InQuad);
                dy[0] = r.get(ms);
            }
            "launch" => {
                // Il selettore scivola dall'icona FILE (0) alla CALC (2), senza salti.
                shell.sel = 2;
                let (from, to) = (icon_cell(0), icon_cell(2));
                let (mut x, mut y) = (Reactive::new(from.x), Reactive::new(from.y));
                x.set(to.x, 0, GLIDE_MS, Ease::OutCubic);
                y.set(to.y, 0, GLIDE_MS, Ease::OutCubic);
                sel = Some(Rect::new(x.get(ms), y.get(ms), to.w, to.h));
            }
            "open" => {
                shell.wm.open(3, "CALCOLATRICE", 80, 60, LW, LH);
                let mut r = Reactive::new(WIN_DY);
                r.set(0, 0, OPEN_MS, Ease::OutCubic);
                dy[0] = r.get(ms);
            }
            _ => {}
        }
    }
    let m = if ms.is_some() { Motion { win_dy: &dy, sel } } else { STILL };

    // Budget di frame: 30 render misurati, µs per frame (target 60 Hz).
    let mut bud = Budget::new(FRAME_US);
    let mut present_us = 0u64;
    for _ in 0..30 {
        let t0 = std::time::Instant::now();
        draw_motion(&mut v, &shell, &m);
        bud.record(t0.elapsed().as_micros() as u32);
        v.dirty.add(Rect::new(0, 0, w, h));
        let t1 = std::time::Instant::now();
        v.present();
        present_us += t1.elapsed().as_micros() as u64;
    }
    println!("frame: media {} µs, max {} µs, oltre 16667 µs: {}/{}", bud.avg_us(), bud.max_us, bud.over, bud.frames);
    println!("present (copia zone sporche): media {} µs", present_us / 30);

    let mut f = std::io::BufWriter::new(std::fs::File::create(out).expect("file"));
    write!(f, "P6\n{w} {h}\n255\n").unwrap();
    for px in &v.buf {
        f.write_all(&[(px >> 16) as u8, (px >> 8) as u8, *px as u8]).unwrap();
    }
    println!("scala {} -> {out}", v.s);
}
