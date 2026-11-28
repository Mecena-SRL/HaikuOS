// Misura il disegno procedurale della scrivania (con una finestra aperta, quindi barra
// e pulsanti min/max/chiudi generati da layout + Canvas) contro un frame salvato.
// Eseguire: cargo test --release --test chrome_bench -- --ignored --nocapture
mod common;
use common::Buf;
use haiku_gui::render::draw;
use haiku_gui::{Icon, Key, Shell, Tok};
use std::{hint::black_box, time::Instant};

static ICONS: [Icon; 2] = [
    Icon { label: "FILE", app: 1, title: "FILE", w: 60, h: 50 },
    Icon { label: "TERM", app: 2, title: "TERM", w: 40, h: 30 },
];

#[test]
#[ignore]
fn bench_generated_vs_stored() {
    const N: u32 = 500;
    for &(w, h) in &[(240i32, 160i32), (640, 480)] {
        let mut s = Shell::new(&ICONS, w, h);
        s.key(Key::A);
        let mut frame = Buf::new(w, h);
        draw(&mut frame, &s);
        let stored: Vec<Tok> = frame.px.clone();

        let mut b = Buf::new(w, h);
        let t = Instant::now();
        for _ in 0..N {
            draw(black_box(&mut b), black_box(&s));
        }
        let gen_us = t.elapsed().as_secs_f64() * 1e6 / N as f64;

        let t = Instant::now();
        for _ in 0..N {
            b.px.copy_from_slice(black_box(&stored));
        }
        let blit_us = t.elapsed().as_secs_f64() * 1e6 / N as f64;

        println!(
            "{w}x{h} logico: generato {gen_us:.2} us/frame | salvato (copia) {blit_us:.2} us/frame | RAM salvato {} B | RAM generatore: 0 B extra",
            stored.len() * core::mem::size_of::<Tok>()
        );
    }
}
