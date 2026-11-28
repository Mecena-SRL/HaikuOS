//! Demo host: un pilota automatico percorre ogni livello. Stampa il costo per frame
//! (logica + rendering) e salva un'immagine PPM del livello 1.
//! Uso: `cargo run -p platformer --release -- [out.ppm]`
//! Gioco a terminale: `--play` (invio avvia, ogni riga avanza di 60 frame, `p` pausa, `q` esce).
//! Fotogrammi con menu: `--shot-title|--shot-play|--shot-pause <out.ppm>`.
mod screen;

use platformer::{bot::Bot, levels::LEVELS, render, Game, State, VIEW_H, VIEW_W};
use screen::{menu_text, on_menu_key, overlay, Next, Screen};
use std::io::BufRead;
use std::{
    fs::File,
    io::Write,
    time::{Duration, Instant},
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--play") => return interactive(),
        Some(flag @ ("--shot-title" | "--shot-play" | "--shot-pause")) => {
            let screen = match flag {
                "--shot-title" => Screen::Title,
                "--shot-play" => Screen::Play,
                _ => Screen::Pause,
            };
            return shot(args.get(2).map_or("platformer.ppm", String::as_str), screen);
        }
        _ => {}
    }
    let out = args
        .get(1)
        .cloned()
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

/// Un passo di gioco con il pilota; a fine livello passa al successivo, a sconfitta o vittoria riparte.
fn advance(g: &mut Game, bot: &mut Bot, k: &mut usize) {
    if g.state != State::Playing {
        *k = match g.state {
            State::Over | State::Won => 0,
            _ => (*k + 1) % LEVELS.len(),
        };
        *g = Game::from_src(LEVELS[*k]);
        *bot = Bot::default();
        return;
    }
    let input = bot.input(g);
    g.update(input);
}

fn interactive() {
    let (mut k, mut g, mut bot) = (0usize, Game::from_src(LEVELS[0]), Bot::default());
    let mut fb = vec![0u32; (VIEW_W * VIEW_H) as usize];
    let mut screen = Screen::Title;
    menu_text(screen);
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let key = line.trim();
        match screen {
            Screen::Title | Screen::Pause => match on_menu_key(screen, key) {
                Next::Go(s) => screen = s,
                Next::Quit => break,
                Next::Stay => {}
            },
            Screen::Play => match key {
                "p" => screen = Screen::Pause,
                "q" => break,
                _ => (0..60).for_each(|_| advance(&mut g, &mut bot, &mut k)),
            },
        }
        render::draw(&g, &mut fb);
        overlay(&mut fb, screen);
        if screen == Screen::Play {
            println!(
                "livello {}: {:?}, punteggio {}, vite {}",
                k + 1,
                g.state,
                g.score,
                g.lives
            );
        } else {
            menu_text(screen);
        }
    }
}

fn shot(path: &str, screen: Screen) {
    let (mut k, mut g, mut bot) = (0usize, Game::from_src(LEVELS[0]), Bot::default());
    for _ in 0..300 {
        advance(&mut g, &mut bot, &mut k);
    }
    let mut fb = vec![0u32; (VIEW_W * VIEW_H) as usize];
    render::draw(&g, &mut fb);
    overlay(&mut fb, screen);
    write_ppm(path, &fb);
    println!("fotogramma {VIEW_W}x{VIEW_H} scritto in {path}");
}

fn write_ppm(path: &str, fb: &[u32]) {
    let mut f = File::create(path).expect("creazione file");
    // Ingrandimento x2 (vicino più prossimo) per leggere i menu.
    let (w, h) = (VIEW_W as usize, VIEW_H as usize);
    write!(f, "P6\n{} {}\n255\n", 2 * w, 2 * h).unwrap();
    let bytes: Vec<u8> = (0..4 * w * h)
        .flat_map(|i| {
            let (ox, oy) = (i % (2 * w), i / (2 * w));
            let c = fb[(oy / 2) * w + ox / 2];
            [(c >> 16) as u8, (c >> 8) as u8, c as u8]
        })
        .collect();
    f.write_all(&bytes).unwrap();
}
