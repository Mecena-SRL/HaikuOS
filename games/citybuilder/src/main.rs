//! Città-impero da terminale. Comandi: `t` fine turno, `b <edificio> <x> <y>`,
//! `r` ricerca irrigazione, `q` esci. Edifici: casa, fattoria, legno, cava, mercato, biblioteca.

use citybuilder::menu::{on_menu_key, Next, Screen};
use citybuilder::{Building, Game, Status, Terrain, WIN_POP};
use std::io::{self, BufRead};

fn main() {
    let mut g = Game::new(24, 12, 0x4841494B55);
    // `--shot <file.ppm>` (partita), `--shot-title` e `--shot-pause`: fotogrammi grafici, senza terminale.
    let args: Vec<String> = std::env::args().collect();
    if let [_, flag, path, rest @ ..] = args.as_slice() {
        let screen = match flag.as_str() {
            "--shot" => Some(Screen::Play),
            "--shot-title" => Some(Screen::Title),
            "--shot-pause" => Some(Screen::Pause),
            _ => None,
        };
        if let Some(screen) = screen {
            // Facoltativo `W H`: risoluzione della scena vettoriale (default 1920x1080).
            let (w, h) = match rest {
                [w, h] => (w.parse().expect("larghezza"), h.parse().expect("altezza")),
                _ => (1920, 1080),
            };
            shot(&mut g, path, screen, w, h);
            return;
        }
    }
    let mut screen = Screen::Title;
    menu_text(screen);
    for line in io::stdin().lock().lines().map_while(Result::ok) {
        let key = line.trim();
        if screen != Screen::Play {
            match on_menu_key(screen, key) {
                Next::Go(s) => screen = s,
                Next::Quit => break,
                Next::Stay => {}
            }
            if screen == Screen::Play {
                draw(&g);
            } else {
                menu_text(screen);
            }
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.as_slice() {
            ["p"] => {
                screen = Screen::Pause;
                draw(&g);
                menu_text(screen);
                continue;
            }
            ["t"] => g.end_turn(),
            ["r"] => {
                if !g.player.research_irrigation() {
                    println!("ricerca non disponibile");
                }
            }
            ["b", name, x, y] => {
                match (parse_building(name), x.parse::<usize>(), y.parse::<usize>()) {
                    (Some(b), Ok(x), Ok(y)) if x < g.map.w && y < g.map.h => {
                        let i = g.map.idx(x, y);
                        if let Err(e) = g.player.build(&g.map, i, b) {
                            println!("impossibile: {e:?}");
                        }
                    }
                    _ => println!("coordinate o edificio non validi"),
                }
            }
            ["q"] => break,
            _ => println!(
                "comandi: t | b <casa|fattoria|legno|cava|mercato|biblioteca> <x> <y> | r | q"
            ),
        }
        draw(&g);
        match g.status() {
            Status::Won => {
                println!("Vittoria: {WIN_POP} cittadini!");
                break;
            }
            Status::Lost => {
                println!("Sconfitta.");
                break;
            }
            Status::Playing => {}
        }
    }
}

fn parse_building(s: &str) -> Option<Building> {
    Some(match s {
        "casa" => Building::House,
        "fattoria" => Building::Farm,
        "legno" => Building::Lumber,
        "cava" => Building::Quarry,
        "mercato" => Building::Market,
        "biblioteca" => Building::Library,
        _ => return None,
    })
}

/// Solo rendering: legge lo stato, non lo modifica.
fn glyph(g: &Game, i: usize) -> char {
    if let Some(b) = g.player.building_at(i) {
        return match b {
            Building::House => 'H',
            Building::Farm => 'F',
            Building::Lumber => 'L',
            Building::Quarry => 'Q',
            Building::Market => 'M',
            Building::Library => 'B',
        };
    }
    if g.rival.building_at(i).is_some() {
        return 'x';
    }
    match g.map.terrain(i) {
        Some(Terrain::Grass) => '.',
        Some(Terrain::Forest) => 'T',
        Some(Terrain::Mountain) => '^',
        Some(Terrain::Water) => '~',
        None => '?',
    }
}

/// Testo dei menu di avvio e pausa (il terminale non ha grafica).
fn menu_text(s: Screen) {
    match s {
        Screen::Title => println!("CITTÀ-IMPERO\nINVIO per iniziare, q per uscire"),
        Screen::Pause => println!("PAUSA: r riprendi, q esci"),
        Screen::Play => {}
    }
}

fn draw(g: &Game) {
    for y in 0..g.map.h {
        let row: String = (0..g.map.w).map(|x| glyph(g, g.map.idx(x, y))).collect();
        println!("{row}");
    }
    let s = g.player.stock;
    println!(
        "pop {}/{WIN_POP}  cibo {}  legno {}  pietra {}  oro {}  conoscenza {}  turno {}",
        g.player.citizens, s.food, s.wood, s.stone, s.gold, g.player.knowledge, g.turns
    );
    println!("IA (x): pop {}", g.rival.citizens);
}

/// Gioca alcuni turni con poche costruzioni valide e scrive un PPM.
fn shot(g: &mut Game, path: &str, screen: Screen, w: usize, h: usize) {
    for _ in 0..6 {
        g.end_turn();
    }
    let plan = [
        Building::Farm,
        Building::House,
        Building::Lumber,
        Building::Quarry,
        Building::Farm,
    ];
    for b in plan {
        let spot = (0..g.map.w * g.map.h).find(|&i| {
            g.player.building_at(i).is_none()
                && g.map.terrain(i).is_some_and(|t| t != Terrain::Water)
                && b.terrain()
                    .is_none_or(|need| g.map.terrain(i) == Some(need))
        });
        if let Some(i) = spot {
            let _ = g.player.build(&g.map, i, b);
        }
        g.end_turn();
    }
    let mut frame = citybuilder::view::render(g, w, h);
    citybuilder::view::overlay(&mut frame, screen);
    let mut out = std::fs::File::create(path).expect("impossibile creare il file");
    frame.write_ppm(&mut out).expect("scrittura fallita");
    println!("fotogramma {}x{} scritto in {path}", frame.w, frame.h);
}
