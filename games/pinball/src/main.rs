//! Host: senza argomenti misura 600000 frame (bot); con `N` ne misura N; con `--play` gioca a terminale
//! (titolo, gioco, pausa, fine); con `--shot-title|--shot-play|--shot-pause|--shot-over <file.ppm> [W H]`
//! scrive un fotogramma vettoriale (default 1920×1080).
//! Il rendering sta in `screen.rs` (std); il core `pinball` è `no_std`.

mod screen;

use pinball::{Input, Pinball, State, BUMPERS, FP, TABLE_H, TABLE_W};
use screen::{menu_text, on_menu_key, render, Next, Screen};
use std::io::{self, BufRead};
use std::time::Instant;

const COLS: usize = 40;
const ROWS: usize = 30;

fn bot(g: &Pinball, charge: &mut u32, frame: u32) -> Input {
    if g.state == State::Ready {
        let mut i = Input::default();
        if *charge < 30 {
            *charge += 1;
            i.plunger = true;
        } else {
            *charge = 0;
        }
        return i;
    }
    let flutter = (frame / 6) % 2 == 0;
    let (mut l, mut r) = (false, false);
    for b in g.balls.iter().filter(|b| b.live && b.pos.1 > 170 * FP) {
        if b.pos.0 < 80 * FP {
            l = true
        } else {
            r = true
        }
    }
    Input { left: l && flutter, right: r && flutter, plunger: false }
}

/// Un frame con il bot. `true` se la partita finisce (lo stato resta `Over`).
fn step(g: &mut Pinball, charge: &mut u32, frame: u32) -> bool {
    g.tick(bot(g, charge, frame));
    g.state == State::Over
}

fn put(grid: &mut [[u8; COLS]; ROWS], x: i64, y: i64, ch: u8) {
    let c = (x / FP * COLS as i64 / TABLE_W).clamp(0, COLS as i64 - 1) as usize;
    let r = (y / FP * ROWS as i64 / TABLE_H).clamp(0, ROWS as i64 - 1) as usize;
    grid[r][c] = ch;
}

fn ascii(g: &Pinball) -> String {
    let mut grid = [[b'.'; COLS]; ROWS];
    for c in BUMPERS {
        put(&mut grid, c.0 * FP, c.1 * FP, b'O');
    }
    for f in &g.flips {
        let t = f.tip();
        for k in 0..=8 {
            put(&mut grid, f.pivot.0 + (t.0 - f.pivot.0) * k / 8, f.pivot.1 + (t.1 - f.pivot.1) * k / 8, b'=');
        }
    }
    for b in g.balls.iter().filter(|b| b.live) {
        put(&mut grid, b.pos.0, b.pos.1, b'o');
    }
    grid.iter().map(|row| std::str::from_utf8(row).unwrap_or("")).collect::<Vec<_>>().join("\n")
}

/// Gioco a terminale: invio avanza di un secondo (60 frame) col bot; `p` pausa, `q` esce.
fn interactive() {
    let mut g = Pinball::new();
    let (mut charge, mut games, mut f) = (0u32, 0u32, 0u32);
    let mut screen = Screen::Title;
    menu_text(screen, &g);
    for line in io::stdin().lock().lines().map_while(Result::ok) {
        let key = line.trim();
        match screen {
            Screen::Play => match key {
                "p" => screen = Screen::Pause,
                "q" => break,
                _ => {
                    for _ in 0..60 {
                        if step(&mut g, &mut charge, f) {
                            games += 1;
                            screen = Screen::Over;
                            break;
                        }
                        f = f.wrapping_add(1);
                    }
                }
            },
            _ => match on_menu_key(screen, key) {
                Next::Go(s) => {
                    if s == Screen::Play && screen == Screen::Over {
                        g.reset();
                    }
                    screen = s;
                }
                Next::Quit => break,
                Next::Stay => {}
            },
        }
        if screen == Screen::Play {
            println!("{}\npunti {}  vite {}  record {}  partite {games}", ascii(&g), g.score, g.lives, g.high);
        } else {
            menu_text(screen, &g);
        }
    }
}

fn shot(path: &str, screen: Screen, size: (usize, usize)) {
    let mut g = Pinball::new();
    let mut charge = 0u32;
    for f in 0..600 {
        if step(&mut g, &mut charge, f) {
            break;
        }
    }
    let im = render(&g, screen, size.0, size.1);
    let mut out = std::fs::File::create(path).expect("impossibile creare il file");
    im.write_ppm(&mut out).expect("scrittura fallita");
    println!("fotogramma {}x{} scritto in {path}", im.w, im.h);
}

fn bench(frames: u32) {
    let mut g = Pinball::new();
    let (mut charge, mut games) = (0u32, 0u32);
    let start = Instant::now();
    for f in 0..frames {
        if step(&mut g, &mut charge, f) {
            games += 1;
            g.reset();
        }
    }
    let ns = start.elapsed().as_nanos() / frames.max(1) as u128;
    println!("{frames} frame, {games} partite finite, record {}", g.high);
    println!("costo: {ns} ns/frame = {:.4}% del budget 60 Hz", ns as f64 / 166_666.0 * 100.0);
    println!("{}", ascii(&g));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.as_slice() {
        [_] => bench(600_000),
        [_, flag] if flag == "--play" => interactive(),
        [_, flag, path, rest @ ..] if flag.starts_with("--shot-") => {
            let size = match rest {
                [w, h] => (w.parse().expect("larghezza"), h.parse().expect("altezza")),
                _ => (1920, 1080),
            };
            match flag.as_str() {
                "--shot-title" => shot(path, Screen::Title, size),
                "--shot-play" => shot(path, Screen::Play, size),
                "--shot-pause" => shot(path, Screen::Pause, size),
                "--shot-over" => shot(path, Screen::Over, size),
                _ => eprintln!("uso: pinball [N] | --play | --shot-title|--shot-play|--shot-pause|--shot-over <file.ppm> [W H]"),
            }
        }
        [_, n] => bench(n.parse().unwrap_or(600_000)),
        _ => eprintln!("uso: pinball [N] | --play | --shot-*<file.ppm> [W H]"),
    }
}
