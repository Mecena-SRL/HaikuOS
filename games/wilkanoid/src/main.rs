//! Host: `wilkanoid shot <cartella>` scrive i PPM delle scene; `wilkanoid sim` gioca in automatico.

use std::{env, fs};
use wilkanoid::{autoplay, render::render, Cmd, Game, Phase};

const SIZES: [(usize, usize, &str); 2] = [(1920, 1080, "1920x1080"), (800, 480, "800x480")];

fn scenes() -> Vec<(&'static str, Game)> {
    let mut v = vec![("titolo", Game::new(1))];

    let mut g = Game::at_level(1, 0);
    g.input(Cmd::Fire);
    for _ in 0..240 {
        autoplay(&mut g);
    }
    v.push(("livello1", g));

    let mut g = Game::at_level(2, 2);
    g.input(Cmd::Fire);
    for _ in 0..300 {
        autoplay(&mut g);
    }
    v.push(("livello3", g));

    let mut g = Game::new(5);
    g.input(Cmd::Fire);
    for _ in 0..1_000_000 {
        if g.phase == Phase::Over {
            break;
        }
        if g.phase == Phase::Ready {
            g.input(Cmd::Fire);
        }
        g.tick();
    }
    v.push(("game-over", g));

    let mut g = Game::new(7);
    g.input(Cmd::Fire);
    for _ in 0..1_000_000 {
        if matches!(g.phase, Phase::Won | Phase::Over) {
            break;
        }
        autoplay(&mut g);
    }
    v.push(("vittoria", g));
    v
}

fn shots(dir: &str) {
    for (name, g) in scenes() {
        for (w, h, tag) in SIZES {
            let mut px = vec![0u32; w * h];
            render(&g, &mut px, w, h);
            let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
            for p in &px {
                out.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
            }
            let path = format!("{dir}/wilkanoid-{name}-{tag}-{w}x{h}.ppm");
            fs::write(&path, out).expect("scrittura fallita");
            println!("{path}");
        }
    }
}

fn sim() {
    let mut g = Game::new(7);
    g.input(Cmd::Fire);
    let mut t = 0u64;
    while !matches!(g.phase, Phase::Won | Phase::Over) && t < 1_000_000 {
        autoplay(&mut g);
        t += 1;
    }
    println!("fase {:?} livello {} punti {} tick {} vite {}", g.phase, g.level + 1, g.score, t, g.lives);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("shot") => shots(args.get(2).map_or(".", String::as_str)),
        Some("sim") => sim(),
        _ => eprintln!("uso: wilkanoid shot <cartella> | sim"),
    }
}
