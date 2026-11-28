//! Host: `gwent` gioca a terminale. Comandi: `a`/`d` sposta la selezione, invio gioca, `p` passa,
//! `click X Y` (coordinate di progetto 1920×1080) gioca o passa col mouse, `q` esce.
//! `gwent --shot <file.ppm> <W> <H> [mosse]` scrive un fotogramma dopo `mosse` turni automatici.

mod view;
#[cfg(feature = "window")]
mod window;

use gwent::ai;
use gwent::{Act, Event, Game};
use std::io::{self, BufRead, Write};

/// Stato di gioco più selezione e riga di stato.
struct Ui {
    g: Game,
    cur: usize,
    msg: String,
}

fn note(ev: Event) -> Option<String> {
    Some(match ev {
        Event::Continue => return None,
        Event::Tie => "Pareggio: il round si ripete.".to_string(),
        Event::RoundWon(0) => "Round tuo.".to_string(),
        Event::RoundWon(_) => "Round dell'avversario.".to_string(),
        Event::MatchWon(0) => "Hai vinto la partita!".to_string(),
        Event::MatchWon(_) => "Ha vinto l'avversario.".to_string(),
    })
}

impl Ui {
    fn new(seed: u32) -> Self {
        let mut ui = Ui { g: Game::new(seed), cur: 0, msg: String::new() };
        ui.ai_turns();
        ui
    }

    /// Fa giocare l'avversario finché non tocca a te o la partita finisce.
    fn ai_turns(&mut self) {
        while self.g.over.is_none() && self.g.turn == 1 {
            let a = ai::choose(&self.g, 1);
            if let Some(ev) = self.g.act(1, a) {
                if let Some(m) = note(ev) {
                    self.msg = m;
                }
            }
        }
        let n = self.g.side[0].hand.n as usize;
        self.cur = self.cur.min(n.saturating_sub(1));
    }

    /// Gioca la carta selezionata, o passa se la mano è vuota.
    fn play_cur(&mut self) -> bool {
        let n = self.g.side[0].hand.n as usize;
        if n == 0 { self.human(Act::Pass) } else { self.human(Act::Play(self.cur)) }
    }

    /// Mossa tua: `Play(cur)` o `Pass`.
    fn human(&mut self, a: Act) -> bool {
        let Some(ev) = self.g.act(0, a) else { return false };
        if let Some(m) = note(ev) {
            self.msg = m;
        }
        self.ai_turns();
        true
    }

    fn print(&self) {
        let g = &self.g;
        println!(
            "ROUND {}  tu {} - avv {}  vittorie {}-{}  meteo:{}{}",
            g.round,
            g.score(0),
            g.score(1),
            g.side[0].wins,
            g.side[1].wins,
            if g.frost { " gelo" } else { "" },
            if g.fog { " nebbia" } else { "" }
        );
        for (board, name) in [(1usize, "avv"), (0, "tu ")] {
            for row in 0..2 {
                let names: Vec<String> = g.side[board].rows[row]
                    .as_slice()
                    .iter()
                    .enumerate()
                    .map(|(i, &s)| format!("{}({})", gwent::cards::DEFS[(s & 0x7f) as usize].name, g.slot_power(board, row, i)))
                    .collect();
                let fila = if row == 0 { "vicina " } else { "distanza" };
                println!("  {name} {fila}: {}", names.join(", "));
            }
        }
        println!("  mano:");
        for (i, &id) in g.side[0].hand.as_slice().iter().enumerate() {
            let d = gwent::cards::DEFS[id as usize];
            let mark = if i == self.cur { ">" } else { " " };
            println!("  {mark}[{i}] {} ({})", d.name, d.power);
        }
        if !self.msg.is_empty() {
            println!("{}", self.msg);
        }
        if g.over.is_none() && g.turn == 0 {
            println!("a/d sposta  invio gioca  p passa  click X Y  q esci");
        }
    }
}

fn shot(path: &str, w: usize, h: usize, moves: usize) {
    let mut ui = Ui::new(1);
    for _ in 0..moves {
        if ui.g.over.is_some() {
            break;
        }
        let who = ui.g.turn as usize;
        let a = ai::choose(&ui.g, who);
        if let Some(ev) = ui.g.act(who as u8, a) {
            if let Some(m) = note(ev) {
                ui.msg = m;
            }
        }
    }
    let p = view::frame(&ui.g, ui.cur, &ui.msg, w, h);
    let mut f = std::fs::File::create(path).expect("file");
    p.im.write_ppm(&mut f).expect("ppm");
    println!("scritto {path} ({w}x{h})");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--shot") {
        let num = |i: usize, d: usize| args.get(i).and_then(|v| v.parse().ok()).unwrap_or(d);
        let path = args.get(1).cloned().unwrap_or_else(|| "gwent.ppm".into());
        shot(&path, num(2, 1920), num(3, 1080), num(4, 0));
        return;
    }
    let seed = args.iter().find_map(|v| v.parse().ok()).unwrap_or(1);
    if args.iter().any(|a| a == "--window") {
        #[cfg(feature = "window")]
        {
            window::run(Ui::new(seed));
            return;
        }
        #[cfg(not(feature = "window"))]
        {
            eprintln!("compilare con --features window per la finestra");
            return;
        }
    }
    let mut ui = Ui::new(seed);
    ui.print();
    for line in io::stdin().lock().lines().map_while(Result::ok) {
        let l = line.trim();
        let ok = match l {
            "q" => break,
            "a" => {
                ui.cur = ui.cur.saturating_sub(1);
                true
            }
            "d" => {
                ui.cur = (ui.cur + 1).min(ui.g.side[0].hand.n.saturating_sub(1) as usize);
                true
            }
            "" => {
                let n = ui.g.side[0].hand.n as usize;
                if n == 0 {
                    ui.human(Act::Pass)
                } else {
                    ui.human(Act::Play(ui.cur))
                }
            }
            "p" => ui.human(Act::Pass),
            _ if l.starts_with("click ") => {
                let v: Vec<i32> = l[6..].split_whitespace().filter_map(|t| t.parse().ok()).collect();
                match (v.first(), v.get(1)) {
                    (Some(&x), Some(&y)) => match view::hit(x, y, ui.g.side[0].hand.n as usize) {
                        Some(view::Hit::Pass) => ui.human(Act::Pass),
                        Some(view::Hit::Card(i)) => {
                            ui.cur = i;
                            ui.human(Act::Play(i))
                        }
                        None => false,
                    },
                    _ => false,
                }
            }
            _ => false,
        };
        if !ok && !l.is_empty() {
            println!("comando non valido");
        }
        if ui.g.over.is_some() {
            ui.print();
            println!("partita finita");
            break;
        }
        ui.print();
        io::stdout().flush().ok();
    }
}
