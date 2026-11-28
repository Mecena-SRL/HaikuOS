//! CLI host dei giochi di carte. Uso:
//!   cards <gioco> <out.ppm> <larghezza> <altezza> [seme]
//! Comandi da stdin (uno per riga): n = nuova, l/u = precedente, r/d = successivo,
//! e = attiva il fuoco, c X Y = clic in pixel, q = fine. Il frame è salvato dopo ogni comando.
use cards::draw::{Pic, Ui, H, W};
use cards::{fr, gwent, na, Game, NEW};
use std::io::{self, BufRead};

fn make(name: &str, seed: u32) -> Option<Box<dyn Game>> {
    Some(match name {
        "klondike" => Box::new(fr::Klondike::new(seed)),
        "blackjack" => Box::new(fr::Blackjack::new(seed)),
        "poker" => Box::new(fr::Poker::new(seed)),
        "otto" => Box::new(fr::Otto::new(seed)),
        "guerra" => Box::new(fr::War::new(seed)),
        "scopa" => Box::new(na::Scopa::new(seed)),
        "briscola" => Box::new(na::Trick::new(seed)),
        "tressette" => Box::new(na::Trick::tressette(seed)),
        "settemezzo" => Box::new(na::SetteMezzo::new(seed)),
        "gwent" => Box::new(gwent::Gwent::new(seed)),
        _ => return None,
    })
}

fn frame(g: &dyn Game, ui: &mut Ui, px: &mut [u32], w: usize, h: usize, out: &str) -> io::Result<()> {
    ui.reset();
    let mut pic = Pic { px, w: w as i32, h: h as i32 };
    g.draw(&mut pic, ui);
    ui.focus_frame(&mut pic);
    let mut buf = format!("P6\n{w} {h}\n255\n").into_bytes();
    for p in px.iter() {
        buf.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
    }
    std::fs::write(out, buf)
}

fn main() -> io::Result<()> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 4 {
        eprintln!("uso: cards <klondike|blackjack|poker|otto|guerra|scopa|briscola|tressette|settemezzo|gwent> <out.ppm> <w> <h> [seme]");
        return Ok(());
    }
    let seed: u32 = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(12345);
    let Some(mut g) = make(&a[0], seed) else {
        eprintln!("gioco sconosciuto: {}", a[0]);
        return Ok(());
    };
    let out = &a[1];
    let (w, h): (usize, usize) = (a[2].parse().unwrap_or(1920), a[3].parse().unwrap_or(1080));
    let mut px = vec![0u32; w * h];
    let mut ui = Ui::new();
    frame(g.as_ref(), &mut ui, &mut px, w, h, out)?;

    for line in io::stdin().lock().lines() {
        let line = line?;
        let t: Vec<&str> = line.split_whitespace().collect();
        let act = match t.as_slice() {
            ["n"] => Some(NEW),
            ["e"] => ui.fire(),
            ["l"] | ["u"] => {
                ui.prev();
                None
            }
            ["r"] | ["d"] => {
                ui.next();
                None
            }
            ["c", x, y] => match (x.parse::<i32>(), y.parse::<i32>()) {
                (Ok(x), Ok(y)) => ui.click(x * W / w as i32, y * H / h as i32),
                _ => None,
            },
            ["q"] => break,
            _ => None,
        };
        if let Some(a) = act {
            g.act(a);
        }
        frame(g.as_ref(), &mut ui, &mut px, w, h, out)?;
    }
    Ok(())
}
