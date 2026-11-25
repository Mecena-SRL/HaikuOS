//! Lettura comandi da stdin, una riga per comando (vedi `lib.rs`).
use fontmgr::{exec, AppError};
use haiku_alloc_sim::fonts::FontManager;
use std::io::{self, BufRead};

fn main() {
    let mut fm = FontManager::new();
    let read = |p: &str| std::fs::read(p).map_err(|e| e.to_string());
    for line in io::stdin().lock().lines() {
        let line = line.unwrap_or_default();
        match exec(&mut fm, &line, &read) {
            Ok(msg) if msg.is_empty() => {}
            Ok(msg) => println!("{msg}"),
            Err(e) => println!("errore: {e:?}"),
        }
    }
}
