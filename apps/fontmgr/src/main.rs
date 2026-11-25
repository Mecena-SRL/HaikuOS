//! Lettura comandi da stdin, una riga per comando (vedi `lib.rs`).
use fontmgr::exec;

use std::io::{self, BufRead};

fn main() {
    let mut s = fontmgr::Session::new();
    let read = |p: &str| std::fs::read(p).map_err(|e| e.to_string());
    for line in io::stdin().lock().lines() {
        let line = line.unwrap_or_default();
        match exec(&mut s, &line, &read) {
            Ok(msg) if msg.is_empty() => {}
            Ok(msg) => println!("{msg}"),
            Err(e) => println!("errore: {e:?}"),
        }
    }
}
