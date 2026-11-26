//! Scacchi da terminale. Scrivi le mosse tipo "e2e4", una per riga.

use chess::{Game, Move, Status, BLACK, WHITE};
use std::io::{self, BufRead};

const SYM: &[u8; 7] = b".PNBRQK";

fn main() {
    let mut g = Game::new();
    print_board(&g);
    for line in io::stdin().lock().lines().map_while(Result::ok) {
        let Some(m) = Move::parse(line.trim()) else {
            println!("formato non valido (es. e2e4)");
            continue;
        };
        if !g.play(m) {
            println!("mossa illegale");
            continue;
        }
        print_board(&g);
        match g.status() {
            Status::Checkmate => {
                println!("scacco matto: vince {}", if g.turn == WHITE { "nero" } else { "bianco" });
                break;
            }
            Status::Stalemate => {
                println!("stallo, patta");
                break;
            }
            Status::Check => println!("scacco"),
            Status::Playing => {}
        }
    }
}

fn print_board(g: &Game) {
    for y in (0..8).rev() {
        let row: String = (0..8)
            .map(|x| {
                let p = g.board.cells[y][x];
                let c = SYM[(p & 7) as usize] as char;
                if p & BLACK != 0 { c.to_ascii_lowercase() } else { c }
            })
            .map(|c| format!("{c} "))
            .collect();
        println!("{} {row}", y + 1);
    }
    println!("  a b c d e f g h");
    println!("turno: {}", if g.turn == WHITE { "bianco" } else { "nero" });
}
