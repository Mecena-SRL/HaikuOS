//! Città-impero da terminale. Comandi: `t` fine turno, `b <edificio> <x> <y>`,
//! `r` ricerca irrigazione, `q` esci. Edifici: casa, fattoria, legno, cava, mercato, biblioteca.

use citybuilder::{Building, Game, Status, Terrain, WIN_POP};
use std::io::{self, BufRead};

#[path = "../../hd_shot.rs"]
mod hd_shot;

fn main() {
    let mut g = Game::new(24, 12, 0x4841494B55);
    // `--shot <file.ppm>`: partita scriptata e un fotogramma grafico, senza terminale.
    let args: Vec<String> = std::env::args().collect();
    if let [_, flag, path, rest @ ..] = args.as_slice() {
        if flag == "--shot" {
            let hd = match rest {
                [w, h] => Some((w.parse().expect("larghezza"), h.parse().expect("altezza"))),
                _ => None,
            };
            shot(&mut g, path, hd);
            return;
        }
    }
    draw(&g);
    for line in io::stdin().lock().lines().map_while(Result::ok) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.as_slice() {
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
fn shot(g: &mut Game, path: &str, hd: Option<(usize, usize)>) {
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
    let frame = citybuilder::view::render(g);
    let mut out = std::fs::File::create(path).expect("impossibile creare il file");
    match hd {
        Some((ow, oh)) => hd_shot::write_fit(&mut out, frame.w, frame.h, &frame.px, ow, oh).expect("scrittura fallita"),
        None => frame.write_ppm(&mut out).expect("scrittura fallita"),
    }
    println!("fotogramma {}x{} scritto in {path}", frame.w, frame.h);
}
