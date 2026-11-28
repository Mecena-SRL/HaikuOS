//! Demo host: un pilota automatico percorre ogni livello. Stampa il costo per frame
//! (logica + rendering) e salva un'immagine PPM del livello 1.
//! Uso: `cargo run -p platformer --release -- [out.ppm]`
//! Gioco a terminale: `--play` (invio avvia, ogni riga avanza di 60 frame, `p` pausa, `q` esce).
//! Fotogrammi con menu: `--shot-title|--shot-play|--shot-pause <out.ppm>`.
mod screen;

use platformer::render::Fb;
/// Risoluzione del banco di misura e del terminale (5 pollici).
const BENCH_W: usize = 800;
const BENCH_H: usize = 480;

use platformer::{bot::Bot, levels::LEVELS, render, Game, State};
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
            // Facoltativo `W H` dopo il file: risoluzione di uscita (predefinita 1920x1080).
            let size = match args.get(3..5) {
                Some([w, h]) => Some((w.parse().expect("larghezza"), h.parse().expect("altezza"))),
                _ => None,
            };
            return shot(args.get(2).map_or("platformer.ppm", String::as_str), screen, size);
        }
        _ => {}
    }
    let out = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "platformer.ppm".into());
    let mut fb = vec![0u32; BENCH_W * BENCH_H];
    let mut scr = vec![0u32; render::scratch_len(BENCH_W, BENCH_H)];
    let (mut frames, mut spent) = (0u32, Duration::ZERO);
    for (k, src) in LEVELS.iter().enumerate() {
        let mut g = Game::from_src(src);
        let mut bot = Bot::default();
        let mut n = 0u32;
        while g.state == State::Playing && n < 20_000 {
            let input = bot.input(&g);
            let t = Instant::now();
            g.update(input);
            render::draw(&g, &mut fb, &mut scr, BENCH_W, BENCH_H);
            spent += t.elapsed();
            n += 1;
            if k == 0 && n == 300 {
                write_ppm(&out, &fb, BENCH_W, BENCH_H);
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
    let mut fb = vec![0u32; BENCH_W * BENCH_H];
    let mut scr = vec![0u32; render::scratch_len(BENCH_W, BENCH_H)];
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
                _ => {
                    // Si ferma appena la partita finisce, così Over/Won resta visibile.
                    for _ in 0..60 {
                        advance(&mut g, &mut bot, &mut k);
                        if g.state != State::Playing {
                            break;
                        }
                    }
                }
            },
        }
        render::draw(&g, &mut fb, &mut scr, BENCH_W, BENCH_H);
        overlay(&mut Fb::new(&mut fb, BENCH_W, BENCH_H), screen);
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

fn shot(path: &str, screen: Screen, size: Option<(usize, usize)>) {
    let (mut k, mut g, mut bot) = (0usize, Game::from_src(LEVELS[0]), Bot::default());
    for _ in 0..300 {
        advance(&mut g, &mut bot, &mut k);
    }
    let (w, h) = size.unwrap_or((1920, 1080));
    let mut fb = vec![0u32; w * h];
    let mut scr = vec![0u32; render::scratch_len(w, h)];
    render::draw(&g, &mut fb, &mut scr, w, h);
    overlay(&mut Fb::new(&mut fb, w, h), screen);
    write_ppm(path, &fb, w, h);
    println!("fotogramma {w}x{h} scritto in {path}");
}

fn write_ppm(path: &str, fb: &[u32], w: usize, h: usize) {
    let mut f = File::create(path).expect("creazione file");
    write!(f, "P6\n{w} {h}\n255\n").unwrap();
    let px: Vec<u8> = fb.iter().flat_map(|&c| [(c >> 16) as u8, (c >> 8) as u8, c as u8]).collect();
    f.write_all(&px).unwrap();
}
