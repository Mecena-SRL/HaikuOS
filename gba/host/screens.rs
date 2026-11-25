//! Screenshot del desktop dalla stessa `paint` della ROM. Scrive PPM in $OUT (poi si converte in PNG).
//! `cd gba && rustc --edition 2021 host/screens.rs -o /tmp/screens && OUT=dir /tmp/screens`
#![allow(dead_code)]
#[path = "../src/font.rs"]
mod font;
#[path = "../src/glyph.rs"]
mod glyph;
#[path = "../src/editor.rs"]
mod editor;
#[path = "../src/calc.rs"]
mod calc;
#[path = "../src/calc_ui.rs"]
mod calc_ui;
#[path = "../src/apps/mod.rs"]
mod apps;
#[path = "../src/desktop.rs"]
mod desktop;

use desktop::{Desktop, App, KEY_A, KEY_B, KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_UP, W, H};

fn go(d: &mut Desktop, icona: usize) {
    while d.sel() != icona {
        d.step(KEY_RIGHT);
        d.step(0);
    }
}

fn press(d: &mut Desktop, k: u16) {
    d.step(k);
    d.step(0);
}

fn save(name: &str, d: &Desktop) {
    let mut surf = [0u16; calc_ui::SW * calc_ui::SH];
    d.draw_calc(&mut surf);
    let mut fb = vec![0u16; W * H];
    desktop::paint(&mut fb, d, &surf);
    let dir = std::env::var("OUT").unwrap_or(".".into());
    let mut out = format!("P6\n{} {}\n255\n", W, H).into_bytes();
    for &c in &fb {
        let r = (c & 31) as u32 * 255 / 31;
        let g = ((c >> 5) & 31) as u32 * 255 / 31;
        let b = ((c >> 10) & 31) as u32 * 255 / 31;
        out.extend_from_slice(&[r as u8, g as u8, b as u8]);
    }
    std::fs::write(format!("{}/{}.ppm", dir, name), out).unwrap();
}

fn main() {
    let mut d = Desktop::new();
    save("1_home", &d);

    // Note: due note con testo.
    go(&mut d, 3);
    press(&mut d, KEY_A);
    press(&mut d, KEY_RIGHT); // nuova nota
    // Il carattere in attesa resta dov'è: si conta lo scarto dal precedente (CHARS: " A..Z0..9").
    let mut cur = 0usize;
    for c in "CIAO".bytes() {
        let target = (c - b'A' + 1) as usize;
        for _ in 0..(target + 37 - cur) % 37 {
            press(&mut d, KEY_UP);
        }
        cur = target;
        press(&mut d, KEY_A);
    }
    save("2_note", &d);
    press(&mut d, KEY_B);

    // Orologio: 09:05 con un'ora in più.
    go(&mut d, 4);
    press(&mut d, KEY_A);
    save("3_orologio", &d);
    press(&mut d, KEY_B);

    // Impostazioni: tema 2.
    go(&mut d, 5);
    press(&mut d, KEY_A);
    press(&mut d, KEY_RIGHT);
    save("4_impostazioni", &d);
    let _ = (App::None, KEY_DOWN, KEY_LEFT);
}
