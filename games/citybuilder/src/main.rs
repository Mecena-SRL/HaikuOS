//! Città-impero da terminale. Comandi: `t` fine turno, `b <edificio> <x> <y>`,
//! `r` ricerca irrigazione, `q` esci. Edifici: casa, fattoria, legno, cava, mercato, biblioteca.

use citybuilder::{Building, Game, Status, Terrain, WIN_POP};
use std::io::{self, BufRead};

fn main() {
    let mut g = Game::new(24, 12, 0x4841494B55);
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
