//! Versione host. Uso:
//!   rpg                          gioco da terminale: una riga di tasti per volta
//!                                (w/a/s/d muovi, z = A, x = B, q esci)
//!   rpg shot FILE.ppm W H KEYS   screenshot dopo aver premuto KEYS (es. "zz")

use haiku_editor::gui::Canvas;
use rpg::{draw, world, Button, Game, Mode};
use std::{env, fs, io::{self, BufRead, Write}};

const SEED: u32 = 0x2545_F491;

fn key(c: char) -> Option<Button> {
    Some(match c {
        'w' => Button::Up,
        's' => Button::Down,
        'a' => Button::Left,
        'd' => Button::Right,
        'z' => Button::A,
        'x' => Button::B,
        _ => return None,
    })
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut g = Game::new(SEED);
    if args.get(1).map(String::as_str) == Some("shot") {
        let out = &args[2];
        let (w, h): (usize, usize) = (args[3].parse().expect("W"), args[4].parse().expect("H"));
        for c in args.get(5).map_or("", String::as_str).chars() {
            if let Some(b) = key(c) {
                g.press(b);
            }
        }
        for _ in 0..16 {
            g.tick();
        }
        let mut px = vec![0u32; w * h];
        draw::draw(&mut Canvas { px: &mut px, w, h }, &g);
        let mut data = format!("P6\n{w} {h}\n255\n").into_bytes();
        for p in px {
            data.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
        }
        fs::write(out, data).expect("scrittura ppm");
        println!("screenshot {out}");
        return;
    }
    show(&g);
    for line in io::stdin().lock().lines().map_while(Result::ok) {
        for c in line.trim().chars() {
            if c == 'q' {
                return;
            }
            if let Some(b) = key(c) {
                g.press(b);
                g.tick();
            }
        }
        show(&g);
    }
}

fn show(g: &Game) {
    let mut out = io::stdout().lock();
    if g.mode == Mode::Fight {
        let _ = writeln!(out, "\n{}", g.battle.msg.as_str());
        let _ = writeln!(out, "[{:?}] w/s scegli, z conferma, x indietro", g.battle.phase);
        return;
    }
    for y in 0..world::H {
        let row: String = (0..world::W)
            .map(|x| {
                if (x, y) == (g.x, g.y) {
                    '@'
                } else if world::npc_at(g.map, x, y).is_some() {
                    'N'
                } else {
                    world::tile(g.map, x, y)
                }
            })
            .collect();
        let _ = writeln!(out, "{row}");
    }
    match g.mode {
        Mode::Talk => {
            let _ = writeln!(out, "{}", g.text.split('|').nth(g.page).unwrap_or(""));
            let _ = writeln!(out, "[z per continuare]");
        }
        _ => {
            let _ = writeln!(out, "[wasd muovi, z parla]");
        }
    }
}
