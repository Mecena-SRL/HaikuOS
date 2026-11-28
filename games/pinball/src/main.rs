//! Demo a riga di comando: un bot gioca, si misura il costo per frame e si stampa il tavolo.
//! Il rendering sta qui (std); il core `pinball` è `no_std`.

use pinball::{Input, Pinball, State, BUMPERS, FP, TABLE_H, TABLE_W};
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
        if b.pos.0 < 80 * FP { l = true } else { r = true }
    }
    Input { left: l && flutter, right: r && flutter, plunger: false }
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
    grid.iter()
        .map(|row| std::str::from_utf8(row).unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn main() {
    let frames: u32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(600_000);
    let mut g = Pinball::new();
    let (mut charge, mut games) = (0u32, 0u32);
    let start = Instant::now();
    for f in 0..frames {
        let inp = bot(&g, &mut charge, f);
        g.tick(inp);
        if g.state == State::Over {
            games += 1;
            g.reset();
        }
    }
    let ns = start.elapsed().as_nanos() / frames.max(1) as u128;
    println!("{frames} frame, {games} partite finite, record {}", g.high);
    println!("costo: {ns} ns/frame = {:.4}% del budget 60 Hz", ns as f64 / 166_666.0 * 100.0);
    println!("{}", ascii(&g));
}
