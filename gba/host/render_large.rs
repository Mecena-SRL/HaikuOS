//! Renderer host: disegna la scrivania a 240×160 e la ingrandisce con `scale.rs` su uno schermo grande.
//! Uso: `cd gba && rustc --edition 2021 -O host/render_large.rs -o /tmp/render_large && /tmp/render_large 1920 1080 out.ppm`
//! Scrive un PPM (P6, RGB888). Nessuna modifica alla ROM GBA.
#![allow(dead_code)]
#[path = "../src/fs.rs"]
mod fs;
#[path = "../src/font.rs"]
mod font;
#[path = "../src/terminal.rs"]
mod terminal;
#[path = "../src/ascon.rs"]
mod ascon;
#[path = "../src/alloc.rs"]
mod alloc;
#[path = "../src/monolith.rs"]
mod monolith;
#[path = "../src/calc.rs"]
mod calc;
#[path = "../src/calc_ui.rs"]
mod calc_ui;
#[path = "../src/glyph.rs"]
mod glyph;
#[path = "../src/chess.rs"]
mod chess;
#[path = "../src/media_ui.rs"]
mod media_ui;
#[path = "../src/apps/mod.rs"]
mod apps;
#[path = "../src/editor.rs"]
mod editor;
#[path = "../src/reminders.rs"]
mod reminders;
#[path = "../src/desktop.rs"]
mod desktop;
#[path = "../src/scale.rs"]
mod scale;

use std::io::Write;

/// Colore 15 bit (r | g<<5 | b<<10) in RGB888, come `(v<<3)|(v>>2)`.
fn rgb888(c: u16) -> [u8; 3] {
    let e = |v: u16| {
        let v = (v & 31) as u8;
        (v << 3) | (v >> 2)
    };
    [e(c), e(c >> 5), e(c >> 10)]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let w: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1920);
    let h: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1080);
    let out = args.get(3).map(String::as_str).unwrap_or("desktop.ppm");

    // Stesso ordine di `gba/src/main.rs`: desktop, superfici di calcolatrice e lettore.
    let mut dk = desktop::Desktop::new();
    dk.demo_reminders();
    let mut fb = vec![0u16; desktop::W * desktop::H];
    let mut surf = vec![0u16; calc_ui::SW * calc_ui::SH];
    let mut media = vec![0u16; media_ui::W * media_ui::H];
    let fs = Box::new(fs::Fs::new());
    dk.draw_calc(&mut surf);
    dk.draw_media(&mut media);
    desktop::paint(&mut fb, &dk, &surf, &media, &fs);

    let mut big = vec![0u16; w * h];
    if !scale::upscale(&fb, &mut big, w, h, desktop::BG) {
        eprintln!("misura {w}x{h} troppo piccola per 240x160");
        std::process::exit(1);
    }
    let mut f = std::fs::File::create(out).expect("file");
    write!(f, "P6\n{} {}\n255\n", w, h).unwrap();
    let mut bytes = Vec::with_capacity(w * h * 3);
    for &c in &big {
        bytes.extend_from_slice(&rgb888(c));
    }
    f.write_all(&bytes).unwrap();
    println!("fattore {} -> {}", scale::factor(w, h), out);
}
