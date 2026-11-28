//! Scrivania reale disegnata da `render::draw` su un `Canvas` a piena risoluzione.
//! Stesso percorso dell'x86 (`Scaled` in x86/kernel/src/fb.rs): unità logiche 240×135,
//! scala intera (16 a 3840×2160, 8 a 1920×1080). Plate delle icone via `rect_rounded`.
//! Uso: cargo run --release --features wide-font --example desktop_vector -- 3840 2160 out.ppm [permille]
//! Con `permille` (0..=1000) disegna un frame della transizione `Slide` tra scrivania e calcolatrice.
//! Poi il PPM diventa PNG: `python3 -c "from PIL import Image; Image.open('out.ppm').save('out.png')"`.
#![allow(dead_code)]
#[path = "../../x86/kernel/src/font.rs"]
mod font;

use haiku_gui::anim::{Budget, Shifted, Slide, FRAME_US};
use haiku_gui::icons;
use haiku_gui::layout::Rect;
use haiku_gui::render::{draw, Canvas};
use haiku_gui::{Icon, Shell, Tok};
use std::io::Write;

const LW: i32 = 240;
const LH: i32 = 135;

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
    buf: Vec<u32>,
}

impl Vector {
    /// Riempie il rettangolo fisico [x0,x1)×[y0,y1), ritagliato sullo schermo.
    fn span(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u32) {
        for y in y0.max(0)..y1.min(self.ph) {
            let row = (y * self.pw) as usize;
            for x in x0.max(0)..x1.min(self.pw) {
                self.buf[row + x as usize] = c;
            }
        }
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

    /// Angoli a raggio `radius` (logico) calcolati pixel per pixel in fisico: nitidi a ogni risoluzione.
    fn rect_rounded(&mut self, r: Rect, radius: i32, t: Tok) {
        let s = self.s;
        let (x0, y0, x1, y1) = (r.x * s, r.y * s, (r.x + r.w) * s, (r.y + r.h) * s);
        let rr = radius * s;
        let c = t.rgb();
        for py in y0.max(0)..y1.min(self.ph) {
            for px in x0.max(0)..x1.min(self.pw) {
                let d = |v: i32, a: i32, b: i32| {
                    if v < a + rr {
                        a + rr - 1 - v
                    } else if v >= b - rr {
                        v - (b - rr)
                    } else {
                        0
                    }
                };
                let (dx, dy) = (d(px, x0, x1), d(py, y0, y1));
                if rr == 0 || dx * dx + dy * dy < rr * rr {
                    self.buf[(py * self.pw + px) as usize] = c;
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

    /// Testo: font 5×8 (x86), ogni punto logico diventa un quadrato `s`×`s`.
    fn text(&mut self, x: i32, y: i32, text: &str, t: Tok) {
        let (s, c) = (self.s, t.rgb());
        for (i, &ch) in text.as_bytes().iter().enumerate() {
            for (row, bits) in font::glyph(ch).iter().enumerate() {
                for col in 0..font::GLYPH_W {
                    if bits & (1 << (font::GLYPH_W - 1 - col)) != 0 {
                        let px = x + (i * font::ADVANCE + col) as i32;
                        let py = y + row as i32;
                        self.span(px * s, py * s, px * s + s, py * s + s, c);
                    }
                }
            }
        }
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
    let mut v = Vector { pw: w, ph: h, s: w / LW, buf: vec![0; (w * h) as usize] };
    match a.get(4).and_then(|p| p.parse::<u32>().ok()) {
        Some(p) => {
            // Transizione: scrivania (vecchia) scivola via, calcolatrice (nuova) entra da destra.
            let mut next = Shell::new(&ICONS, LW, LH);
            next.set_clock(10, 30);
            next.wm.open(3, "CALCOLATRICE", 80, 60, LW, LH);
            let (ox, nx) = Slide::start(LW, 0, 1000).offsets(p);
            // Budget di frame: 30 render misurati, µs per frame (target 60 Hz).
            let mut bud = Budget::new(FRAME_US);
            for _ in 0..30 {
                let t0 = std::time::Instant::now();
                draw(&mut Shifted { inner: &mut v, dx: ox, dy: 0 }, &shell);
                draw(&mut Shifted { inner: &mut v, dx: nx, dy: 0 }, &next);
                bud.record(t0.elapsed().as_micros() as u32);
            }
            println!("frame: media {} µs, max {} µs, oltre 16667 µs: {}/{}", bud.avg_us(), bud.max_us, bud.over, bud.frames);
        }
        None => draw(&mut v, &shell),
    }

    let mut f = std::io::BufWriter::new(std::fs::File::create(out).expect("file"));
    write!(f, "P6\n{w} {h}\n255\n").unwrap();
    for px in &v.buf {
        f.write_all(&[(px >> 16) as u8, (px >> 8) as u8, *px as u8]).unwrap();
    }
    println!("scala {} -> {out}", v.s);
}
