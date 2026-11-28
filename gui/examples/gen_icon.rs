//! Prototipo icona generativa (cartella) con unità `u`: 480×272 e 1920×1080.
//! Uso: cargo run --release --example gen_icon -- <dir_out>
//! Scrive <dir_out>/gen_folder_WxH.ppm (poi convertire in PNG con PIL).
#![allow(dead_code)]
use haiku_gui::genicon::{generate_folder, unit_for_scaled, Cache, Entry, USER_SCALES};
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

impl Canvas for Img {
    fn fill(&mut self, r: Rect, t: Tok) {
        self.rect_rounded(r, 0, t);
    }
    fn text(&mut self, _x: i32, _y: i32, _s: &str, _t: Tok) {}
    /// Angoli: pixel nel quadrato d'angolo di lato `rad` ma fuori dal cerchio sono saltati.
    fn rect_rounded(&mut self, r: Rect, rad: i32, t: Tok) {
        let c = rgb(t);
        let (cx0, cx1) = (r.x + rad, r.x + r.w - rad - 1);
        let (cy0, cy1) = (r.y + rad, r.y + r.h - rad - 1);
        for y in r.y.max(0)..(r.y + r.h).min(self.h) {
            for x in r.x.max(0)..(r.x + r.w).min(self.w) {
                let dx = if x < cx0 { cx0 - x } else if x > cx1 { x - cx1 } else { 0 };
                let dy = if y < cy0 { cy0 - y } else if y > cy1 { y - cy1 } else { 0 };
                if dx * dx + dy * dy <= rad * rad {
                    self.px[(y * self.w + x) as usize] = c;
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
    // Ogni schermo × ogni scala utente (100/125/150 %).
    let screens: Vec<(i32, i32, i32)> = [(480, 272), (1920, 1080)]
        .iter()
        .flat_map(|&(w, h)| USER_SCALES.iter().map(move |&p| (w, h, p)))
        .collect();
    let mut cache: Cache<6> = Cache::new();

    for (si, (sw, sh, pct)) in screens.iter().copied().enumerate() {
        let unit = unit_for_scaled(sw, sh, pct);
        let s = unit.icon();
        assert!(s < sh, "icona fuori schermo");
        let mut img = Img { w: sw, h: sh, px: vec![rgb(Tok::T0); (sw * sh) as usize] };
        // Icona al centro, padding 2 u dal bordo (s/8 = 2 u).
        let origin = Rect::new(sw / 2 - s / 2, sh / 2 - s / 2, s, s);
        let e = cache.folder(si, s, s);
        draw(&mut img, origin, &e);
        // Dimensioni risultanti delle parti.
        let parts: Vec<String> = e.prims[..e.n]
            .iter()
            .map(|p| format!("{}x{}@r{}", p.r.w, p.r.h, p.radius))
            .collect();
        println!(
            "{sw}x{sh} {pct}%: u={} classe={:?} icona={}px ({}u) LOD={:?} prim={} parti=[{}] testo={}px pad={}px gap={}px",
            unit.u, unit.class, s, s / unit.u, e.lod, e.n, parts.join(" "), unit.text_px(), 2 * unit.u, unit.u
        );
        let name = format!("{dir}/gen_folder_{sw}x{sh}_{pct}.ppm");
        let mut out = format!("P6\n{sw} {sh}\n255\n").into_bytes();
        for c in &img.px {
            out.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, *c as u8]);
        }
        std::fs::write(&name, out).expect("scrittura ppm");
    }

    // Secondo frame: stessa dimensione → cache.
    for (si, (sw, sh, pct)) in screens.iter().copied().enumerate() {
        let s = unit_for_scaled(sw, sh, pct).icon();
        black_box(cache.folder(si, s, s));
    }
    println!("cache: hit {} miss {}", cache.hits, cache.misses);

    const N: u32 = 1_000_000;
    let t = Instant::now();
    for _ in 0..N {
        black_box(generate_folder(black_box(128), black_box(128)));
    }
    println!("genera 128px: {:.1} ns", t.elapsed().as_nanos() as f64 / N as f64);
    let mut c: Cache<1> = Cache::new();
    c.folder(0, 128, 128);
    let t = Instant::now();
    for _ in 0..N {
        black_box(c.folder(0, black_box(128), black_box(128)));
    }
    println!("cache hit 128px: {:.1} ns", t.elapsed().as_nanos() as f64 / N as f64);
}
