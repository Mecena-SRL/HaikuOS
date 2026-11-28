//! Host: senza argomenti misura 600000 frame (bot); con `N` ne misura N; con `--play` gioca a terminale
//! (titolo, gioco, pausa); con `--shot-title|--shot-play|--shot-pause <file.ppm>` scrive un fotogramma.
//! Il rendering sta qui (std); il core `pinball` è `no_std`.

mod screen;

use pinball::{Input, Pinball, State, BUMPERS, FP, TABLE_H, TABLE_W};
use screen::{frame, menu_text, on_menu_key, Next, Screen};
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
    Input {
        left: l && flutter,
        right: r && flutter,
        plunger: false,
    }
}

/// Un frame di simulazione con il bot; a fine partita riparte e conta la partita.
fn step(g: &mut Pinball, charge: &mut u32, frame: u32, games: &mut u32) {
    let inp = bot(g, charge, frame);
    g.tick(inp);
    if g.state == State::Over {
        *games += 1;
        g.reset();
    }
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
            put(
                &mut grid,
                f.pivot.0 + (t.0 - f.pivot.0) * k / 8,
                f.pivot.1 + (t.1 - f.pivot.1) * k / 8,
                b'=',
            );
        }
    }
    for b in g.balls.iter().filter(|b| b.live) {
        put(&mut grid, b.pos.0, b.pos.1, b'o');
    }
    grid.iter()
        .map(|row| std::str::from_utf8(row).unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Gioco a terminale: invio avvia, ogni riga in gioco avanza di un secondo (60 frame), `p` pausa, `q` esce.
fn interactive() {
    let mut g = Pinball::new();
    let (mut charge, mut games, mut f) = (0u32, 0u32, 0u32);
    let mut screen = Screen::Title;
    menu_text(screen);
    for line in io::stdin().lock().lines().map_while(Result::ok) {
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
                    for _ in 0..60 {
                        step(&mut g, &mut charge, f, &mut games);
                        f = f.wrapping_add(1);
                    }
                }
            },
        }
        if screen == Screen::Play {
            println!(
                "{}\npunti {}  vite {}  record {}  partite {games}",
                ascii(&g),
                g.score,
                g.lives,
                g.high
            );
        } else {
            menu_text(screen);
        }
    }
}

fn shot(path: &str, screen: Screen) {
    let mut g = Pinball::new();
    let (mut charge, mut games) = (0u32, 0u32);
    for f in 0..600 {
        step(&mut g, &mut charge, f, &mut games);
    }
    let im = frame(&g, screen);
    let mut out = std::fs::File::create(path).expect("impossibile creare il file");
    im.write_ppm(&mut out).expect("scrittura fallita");
    println!("fotogramma {}x{} scritto in {path}", im.w, im.h);
}

fn bench(frames: u32) {
    let mut g = Pinball::new();
    let (mut charge, mut games) = (0u32, 0u32);
    let start = Instant::now();
    for f in 0..frames {
        step(&mut g, &mut charge, f, &mut games);
    }
    let ns = start.elapsed().as_nanos() / frames.max(1) as u128;
    println!("{frames} frame, {games} partite finite, record {}", g.high);
    println!(
        "costo: {ns} ns/frame = {:.4}% del budget 60 Hz",
        ns as f64 / 166_666.0 * 100.0
    );
    println!("{}", ascii(&g));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.as_slice() {
        [_] => bench(600_000),
        [_, flag] if flag == "--play" => interactive(),
        [_, flag, path] if flag.starts_with("--shot-") => match flag.as_str() {
            "--shot-title" => shot(path, Screen::Title),
            "--shot-play" => shot(path, Screen::Play),
            "--shot-pause" => shot(path, Screen::Pause),
            _ => eprintln!("uso: pinball [N] | --shot-title|--shot-play|--shot-pause <file.ppm>"),
        },
        [_, n] => bench(n.parse().unwrap_or(600_000)),
        _ => eprintln!("uso: pinball [N] | --shot-title|--shot-play|--shot-pause <file.ppm>"),
    }
}
