//! Demo host: un pilota automatico percorre ogni livello. Stampa il costo per frame
//! (logica + rendering) e salva un'immagine PPM del livello 1.
//! Uso: `cargo run -p platformer --release -- [out.ppm]`
use platformer::{bot::Bot, levels::LEVELS, render, Game, State, VIEW_H, VIEW_W};
use std::{
    fs::File,
    io::Write,
    time::{Duration, Instant},
};

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "platformer.ppm".into());
    let mut fb = vec![0u32; (VIEW_W * VIEW_H) as usize];
    let (mut frames, mut spent) = (0u32, Duration::ZERO);
    for (k, src) in LEVELS.iter().enumerate() {
        let mut g = Game::from_src(src);
        let mut bot = Bot::default();
        let mut n = 0u32;
        while g.state == State::Playing && n < 20_000 {
            let input = bot.input(&g);
            let t = Instant::now();
            g.update(input);
            render::draw(&g, &mut fb);
            spent += t.elapsed();
            n += 1;
            if k == 0 && n == 300 {
                write_ppm(&out, &fb);
            }
        }
        frames += n;
        println!(
            "livello {}: {:?} dopo {} frame, punteggio {}, vite {}",
            k + 1,
            g.state,
            n,
            g.score,
            g.lives
        );
    }
    let us = spent.as_secs_f64() * 1e6 / frames as f64;
    println!(
        "{:.2} us/frame (logica + render), {:.2}% del budget a 60 Hz",
        us,
        us / 16_666.0 * 100.0
    );
}

fn write_ppm(path: &str, fb: &[u32]) {
    let mut f = File::create(path).expect("creazione file");
    write!(f, "P6\n{} {}\n255\n", VIEW_W, VIEW_H).unwrap();
    let bytes: Vec<u8> = fb
        .iter()
        .flat_map(|&c| [(c >> 16) as u8, (c >> 8) as u8, c as u8])
        .collect();
    f.write_all(&bytes).unwrap();
}
