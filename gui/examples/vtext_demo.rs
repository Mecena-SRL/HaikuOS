//! Demo del font vettoriale: testo a più dimensioni con antialiasing (miscela reale col fondo).
//! Uso: cargo run --release --example vtext_demo -- 3840 2160 out.ppm
//! Poi: python3 -c "from PIL import Image; Image.open('out.ppm').save('out.png')"
use haiku_gui::layout::Rect;
use haiku_gui::palette::ACCENTS;
use haiku_gui::render::Canvas;
use haiku_gui::Tok;
use std::io::Write;

const BG: u32 = 0x1B_1F_2A;

fn color(t: Tok) -> u32 {
    match t {
        Tok::T0 => BG,
        Tok::T1 => 0xF2_F2_F2,
        Tok::T2 => 0xF2_C1_4E,
        Tok::T3 => 0x6E_A8_FE,
        Tok::A(k) => ACCENTS[k as usize],
    }
}

struct Img {
    w: i32,
    h: i32,
    px: Vec<u32>,
}

impl Canvas for Img {
    fn fill(&mut self, r: Rect, t: Tok) {
        for y in r.y.max(0)..(r.y + r.h).min(self.h) {
            for x in r.x.max(0)..(r.x + r.w).min(self.w) {
                self.px[(y * self.w + x) as usize] = color(t);
            }
        }
    }

    // Il demo usa solo `vtext`: `text` resta vuoto.
    fn text(&mut self, _x: i32, _y: i32, _s: &str, _t: Tok) {}

    fn blend(&mut self, x: i32, y: i32, a: u8, t: Tok) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let i = (y * self.w + x) as usize;
        let (s, d, a) = (color(t), self.px[i], a as u32);
        let ch = |sh: u32| {
            let (sv, dv) = ((s >> sh) & 255, (d >> sh) & 255);
            (((sv * a + dv * (255 - a) + 127) / 255) & 255) << sh
        };
        self.px[i] = ch(16) | ch(8) | ch(0);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let w: i32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1920);
    let h: i32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1080);
    let out = args.get(3).cloned().unwrap_or_else(|| "vtext.ppm".into());

    let mut img = Img { w, h, px: vec![BG; (w * h) as usize] };
    let m = w / 16;
    let lines: [(i32, &str, Tok); 6] = [
        (56, "HAIKU OS", Tok::T2),
        (28, "Font vettoriale con antialiasing", Tok::T1),
        (40, "ABCDEFGHIJKLMNOPQRSTUVWXYZ", Tok::T1),
        (40, "0123456789 .:-/", Tok::T3),
        (20, "abcdefghijklmnopqrstuvwxyz", Tok::T1),
        (14, "Scala libera: 14, 20, 28, 40, 56 px per em", Tok::T3),
    ];
    let mut y = h / 10;
    for (size, s, t) in lines {
        img.vtext(m, y, size, s, t);
        y += size * 3 / 2 + 16;
    }

    let mut f = std::fs::File::create(&out).expect("create");
    write!(f, "P6\n{} {}\n255\n", w, h).unwrap();
    let mut bytes = Vec::with_capacity((w * h * 3) as usize);
    for p in &img.px {
        bytes.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
    }
    f.write_all(&bytes).unwrap();
}
