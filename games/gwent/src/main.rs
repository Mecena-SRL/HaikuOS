//! Host: `gwent` gioca a terminale. Comandi: `a`/`d` sposta la selezione, invio gioca (o nuova partita a fine
//! partita), `p` passa (in svantaggio va confermato con un secondo `p`), `click X Y` (coordinate 1920×1080),
//! `h` regole, `n` nuova partita, `q` esce.
//! `gwent --window [seme]` apre la finestra (feature `window`).
//! `gwent --shot <file.ppm> <W> <H> [mosse]` scrive un fotogramma dopo `mosse` turni automatici.
//! `gwent --sim [N]` gioca N partite IA contro IA con i due mazzi, scambiando i lati, e stampa l'equilibrio.

mod view;
#[cfg(feature = "window")]
mod window;

use gwent::cards::{DECK_A, DECK_B, DEFS};
use gwent::ai;
use gwent::{Act, Event, Game, Round};
use std::io::{self, BufRead, Write};

/// Stato di gioco più selezione, riga di stato, seme e conferma del passaggio.
pub struct Ui {
    pub g: Game,
    pub cur: usize,
    pub msg: String,
    seed: u32,
    confirm: bool,
}

fn round_text(r: Round) -> String {
    let who = if r.winner == 0 { "Hai vinto il round." } else { "Vince l'avversario." };
    format!("Round: tu {} - avv {}. {}", r.pts[0], r.pts[1], who)
}

/// Testo per un evento; vuoto se non c'è nulla da dire.
fn note(ev: Event) -> String {
    match ev {
        Event::Continue => String::new(),
        Event::Round(r) => round_text(r),
        Event::MatchWon(0) => "Hai vinto la partita!".to_string(),
        Event::MatchWon(_) => "Ha vinto l'avversario.".to_string(),
    }
}

const HELP: &str = "REGOLE
- Ogni turno giochi una carta o passi. Il round finisce quando passano entrambi.
- Vince il round chi ha piu potenza sul tabellone. Pareggio: vince chi non apre.
- Primo a 2 round vince la partita. Chi perde un round apre il successivo.
- Spia: va nella fila avversaria, pesca 2, i suoi punti vanno all'avversario.
- Guaritore: riporta in mano l'unita piu forte del cimitero.
- Stendardo e Trombettiere: +1 alle unita della loro fila.
- Corno: raddoppia le unita della sua fila.
- Scorch: distrugge l'unita piu forte del tabellone.
- Gelo e Nebbia: la fila colpita scende a potenza 1. Sereno toglie il meteo.
- Schieramento: giocando una carta del gruppo pesca dal mazzo gli altri.";

impl Ui {
    pub fn new(seed: u32) -> Self {
        let mut ui = Ui { g: Game::new(seed), cur: 0, msg: String::new(), seed, confirm: false };
        ui.ai_turns();
        ui
    }

    /// Nuova partita con un seme derivato da quello attuale.
    pub fn restart(&mut self) {
        let s = self.seed.wrapping_mul(1103515245).wrapping_add(12345) | 1;
        *self = Ui::new(s);
    }

    /// Fa giocare l'avversario finché non tocca a te o la partita finisce.
    fn ai_turns(&mut self) {
        while self.g.over.is_none() && self.g.turn == 1 {
            let a = ai::choose(&self.g, 1);
            if a == Act::Pass {
                self.msg = "L'avversario passa.".to_string();
            }
            if let Some(ev) = self.g.act(1, a) {
                let m = note(ev);
                if !m.is_empty() {
                    self.msg = m;
                }
            }
        }
        let n = self.g.side[0].hand.n as usize;
        self.cur = self.cur.min(n.saturating_sub(1));
    }

    /// Gioca la carta selezionata, o passa se la mano è vuota.
    pub fn play_cur(&mut self) -> bool {
        let n = self.g.side[0].hand.n as usize;
        if n == 0 { self.human(Act::Pass) } else { self.human(Act::Play(self.cur)) }
    }

    /// Mossa tua. Passare in svantaggio richiede una seconda conferma.
    pub fn human(&mut self, a: Act) -> bool {
        if self.g.turn != 0 || self.g.over.is_some() {
            return false;
        }
        if a == Act::Pass && self.g.score(0) < self.g.score(1) && !self.confirm {
            self.confirm = true;
            self.msg = "Passando perdi il round. Premi di nuovo P per confermare.".to_string();
            return true;
        }
        self.confirm = false;
        let Some(ev) = self.g.act(0, a) else { return false };
        self.msg = if a == Act::Pass { "Hai passato.".to_string() } else { String::new() };
        let m = note(ev);
        if !m.is_empty() {
            self.msg = m;
        }
        self.ai_turns();
        true
    }

    fn print(&self) {
        let g = &self.g;
        println!("ROUND {}  vittorie: tu {} - avv {}", g.round, g.side[0].wins, g.side[1].wins);
        println!("potenza: tu {} - avv {}", g.score(0), g.score(1));
        for (board, name) in [(1usize, "avv"), (0, "tu ")] {
            for row in 0..2 {
                let names: Vec<String> = g.side[board].rows[row]
                    .as_slice()
                    .iter()
                    .enumerate()
                    .map(|(i, &s)| {
                        let d = DEFS[(s & 0x7f) as usize];
                        let p = g.slot_power(board, row, i);
                        if p == d.power as i32 { format!("{}({})", d.name, p) } else { format!("{}({}/{})", d.name, p, d.power) }
                    })
                    .collect();
                let fila = if row == 0 { "vicina " } else { "distanza" };
                println!("  {name} {fila}: {}", names.join(", "));
            }
        }
        println!("  mano:");
        for (i, &id) in g.side[0].hand.as_slice().iter().enumerate() {
            let d = DEFS[id as usize];
            let mark = if i == self.cur { ">" } else { " " };
            println!("  {mark}[{i}] {} ({}) - {}", d.name, d.power, d.desc);
        }
        if !self.msg.is_empty() {
            println!("{}", self.msg);
        }
        if g.over.is_some() {
            println!("partita finita: invio nuova partita, q esci");
        } else if g.turn == 0 {
            println!("a/d sposta  invio gioca  p passa  h regole  n nuova  q esci");
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
            let m = note(ev);
            if !m.is_empty() {
                ui.msg = m;
            }
        }
    }
    let p = view::frame(&ui.g, ui.cur, &ui.msg, w, h);
    let mut f = std::fs::File::create(path).expect("file");
    p.im.write_ppm(&mut f).expect("ppm");
    println!("scritto {path} ({w}x{h})");
}

/// Partite IA contro IA: ogni seme si gioca due volte, con i mazzi scambiati. Stampa le vittorie per mazzo.
fn sim(n: u32) {
    // Mazzi da variabili d'ambiente per la taratura (`GWENT_A="0,1,..."`), altrimenti quelli di default.
    let parse = |k: &str, d: &[u8]| -> Vec<u8> {
        std::env::var(k).ok().map(|v| v.split(',').filter_map(|t| t.trim().parse().ok()).collect()).unwrap_or_else(|| d.to_vec())
    };
    let (ma, mb) = (parse("GWENT_A", &DECK_A), parse("GWENT_B", &DECK_B));
    let (mut a_wins, mut b_wins, mut unfinished) = (0u32, 0u32, 0u32);
    for seed in 1..=n {
        for swap in [false, true] {
            let (da, db): (&[u8], &[u8]) = if swap { (&mb, &ma) } else { (&ma, &mb) };
            let mut g = Game::with_decks(seed, da, db);
            let mut steps = 0;
            while g.over.is_none() && steps < 5000 {
                let who = g.turn as usize;
                let a = ai::choose(&g, who);
                g.act(who as u8, a);
                steps += 1;
            }
            match g.over {
                Some(w) => {
                    // Il mazzo A è il lato 0 quando non è scambiato, il lato 1 quando lo è.
                    if (w == 0) != swap {
                        a_wins += 1;
                    } else {
                        b_wins += 1;
                    }
                }
                None => unfinished += 1,
            }
        }
    }
    let tot = (2 * n).max(1) as f32;
    println!("mazzo A: {a_wins} vittorie ({:.1}%)  mazzo B: {b_wins} vittorie ({:.1}%)  non finite: {unfinished}", 100.0 * a_wins as f32 / tot, 100.0 * b_wins as f32 / tot);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--shot") => {
            let num = |i: usize, d: usize| args.get(i).and_then(|v| v.parse().ok()).unwrap_or(d);
            let path = args.get(1).cloned().unwrap_or_else(|| "gwent.ppm".into());
            shot(&path, num(2, 1920), num(3, 1080), num(4, 0));
            return;
        }
        Some("--sim") => {
            sim(args.get(1).and_then(|v| v.parse().ok()).unwrap_or(200));
            return;
        }
        _ => {}
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
        if l == "q" {
            break;
        }
        if l == "h" {
            println!("{HELP}");
            io::stdout().flush().ok();
            continue;
        }
        if l == "n" {
            ui.restart();
            ui.print();
            continue;
        }
        let ok = match l {
            "a" => {
                ui.cur = ui.cur.saturating_sub(1);
                true
            }
            "d" => {
                ui.cur = (ui.cur + 1).min(ui.g.side[0].hand.n.saturating_sub(1) as usize);
                true
            }
            "" if ui.g.over.is_some() => {
                ui.restart();
                true
            }
            "" => ui.play_cur(),
            "p" => ui.human(Act::Pass),
            _ if l.starts_with("click ") => {
                let v: Vec<i32> = l[6..].split_whitespace().filter_map(|t| t.parse().ok()).collect();
                match (v.first(), v.get(1)) {
                    (Some(&x), Some(&y)) => match view::hit(x, y, ui.g.side[0].hand.n as usize, ui.cur) {
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
        ui.print();
        io::stdout().flush().ok();
    }
}
