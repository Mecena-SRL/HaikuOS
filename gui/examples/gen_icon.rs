//! Prototipo icona generativa (cartella): 3 dimensioni logiche × 2 schermi, LOD e costi.
//! Uso: cargo run --release --example gen_icon -- <dir_out>
//! Scrive <dir_out>/gen_folder_WxH.ppm; poi: python3 -c "from PIL import Image; Image.open('x.ppm').save('x.png')"
#![allow(dead_code)]
use haiku_gui::genicon::{generate_folder, Cache, Entry};
use haiku_gui::layout::Rect;
use haiku_gui::render::Canvas;
use haiku_gui::Tok;
use std::hint::black_box;
use std::time::Instant;

/// Tavolozza di prova (token → RGB), solo per l'anteprima.
fn rgb(t: Tok) -> u32 {
    match t {
        Tok::T0 => 0xF2F2F7,
        Tok::T1 => 0x3478F6,
        Tok::T2 => 0x5AC8FA,
        _ => 0xFFFFFF,
    }
}

struct Img {
    w: i32,
    h: i32,
    px: Vec<u32>,
}

impl Img {
    fn put(&mut self, x: i32, y: i32, c: u32) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = c;
        }
    }
}

impl Canvas for Img {
    fn fill(&mut self, r: Rect, t: Tok) {
        self.rect_rounded(r, 0, t);
    }
    fn text(&mut self, _x: i32, _y: i32, _s: &str, _t: Tok) {}
    /// Angoli: pixel dentro il quadrato d'angolo di lato `rad` ma fuori dal cerchio sono saltati.
    fn rect_rounded(&mut self, r: Rect, rad: i32, t: Tok) {
        let c = rgb(t);
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                let (cx0, cx1) = (r.x + rad, r.x + r.w - rad - 1);
                let (cy0, cy1) = (r.y + rad, r.y + r.h - rad - 1);
                let dx = if x < cx0 { cx0 - x } else if x > cx1 { x - cx1 } else { 0 };
                let dy = if y < cy0 { cy0 - y } else if y > cy1 { y - cy1 } else { 0 };
                if dx * dx + dy * dy <= rad * rad {
                    self.put(x, y, c);
                }
            }
        }
    }
}

fn draw(img: &mut Img, origin: Rect, e: &Entry) {
    for p in &e.prims[..e.n] {
        let r = Rect::new(origin.x + p.r.x, origin.y + p.r.y, p.r.w, p.r.h);
        img.rect_rounded(r, p.radius, p.t);
    }
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    // Logico 1x: 24 / 48 / 96. Schermo 1080p = 1x, 4K = 2x (stessa icona, più pixel).
    let screens = [(1920, 1080, 1), (3840, 2160, 2)];
    let logical = [24, 48, 96];
    let mut cache: Cache<9> = Cache::new(); // uno slot per (schermo, dimensione)

    for (si, (sw, sh, k)) in screens.into_iter().enumerate() {
        let mut img = Img { w: sw, h: sh, px: vec![rgb(Tok::T0); (sw * sh) as usize] };
        for (i, &l) in logical.iter().enumerate() {
            let s = l * k;
            let e = cache.folder(si * 3 + i, s, s);
            let origin = Rect::new(k * 120 + i as i32 * (k * 200), k * 120, s, s);
            draw(&mut img, origin, &e);
            println!("{sw}x{sh} logico {l:>3} fisico {s:>3} LOD {:?} primitivi {}", e.lod, e.n);
        }
        let name = format!("{dir}/gen_folder_{sw}x{sh}.ppm");
        let mut out = format!("P6\n{sw} {sh}\n255\n").into_bytes();
        for c in &img.px {
            out.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, *c as u8]);
        }
        std::fs::write(&name, out).expect("scrittura ppm");
        println!("scritto {name}");
    }

    // Secondo frame sugli stessi schermi: tutte le icone dalla cache.
    for (si, (_sw, _sh, k)) in screens.into_iter().enumerate() {
        for (i, &l) in logical.iter().enumerate() {
            black_box(cache.folder(si * 3 + i, l * k, l * k));
        }
    }
    println!("cache: hit {} miss {}", cache.hits, cache.misses);
    println!("byte per voce di cache: {}", std::mem::size_of::<Entry>());
    println!("byte bitmap RGBA 96x96: {}", 96 * 96 * 4);

    // Costi: generazione da formula, lookup in cache, disegno (96 px fisici, 1e6 volte).
    const N: u32 = 1_000_000;
    let t = Instant::now();
    for _ in 0..N {
        black_box(generate_folder(black_box(96), black_box(96)));
    }
    let gen_ns = t.elapsed().as_nanos() as f64 / N as f64;

    let mut c: Cache<1> = Cache::new();
    c.folder(0, 96, 96);
    let t = Instant::now();
    for _ in 0..N {
        black_box(c.folder(0, black_box(96), black_box(96)));
    }
    let hit_ns = t.elapsed().as_nanos() as f64 / N as f64;

    let e = generate_folder(96, 96);
    let mut img = Img { w: 96, h: 96, px: vec![0; 96 * 96] };
    let t = Instant::now();
    for _ in 0..N / 10 {
        draw(&mut img, Rect::new(0, 0, 96, 96), black_box(&e));
    }
    let draw_ns = t.elapsed().as_nanos() as f64 / (N / 10) as f64;

    println!("genera (formula) 96px: {gen_ns:.1} ns");
    println!("cache hit 96px:        {hit_ns:.1} ns");
    println!("disegno 4 primitivi:   {draw_ns:.1} ns");
}
