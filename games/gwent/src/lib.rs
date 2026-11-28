//! Gwent in stile console: regole, `no_std`, senza allocazione.
//! Due file per giocatore (vicina e a distanza), meteo globale. Ogni turno si gioca una carta o si passa;
//! il round finisce quando passano entrambi. Vince il round chi ha più potenza. In pareggio vince chi non apre.
//! Al meglio dei tre: vince chi arriva a 2 round. Chi perde un round apre il successivo.

#![cfg_attr(not(test), no_std)]

pub mod ai;
pub mod cards;
pub mod run;

use cards::{Kind, Row, DECK_A, DECK_B, DEFS};

pub const DECK: usize = 22;
pub const HAND: usize = 16;
/// Carte in mano all'inizio di partita.
pub const HAND_START: usize = 10;
/// Carte pescate a ogni round.
pub const DRAW_ROUND: usize = 2;
/// Oltre questo limite una carta pescata va al cimitero.
pub const HAND_CAP: usize = 12;
/// Posti per fila (22 carte per giocatore, anche con quelle dell'avversario).
pub const ROW: usize = 32;
/// Round vinti per vincere la partita (al meglio dei tre).
pub const WINS: u8 = 2;
/// Jolly "Sprone": +1 moltiplicatore alla fila che ha almeno 3 carte.
pub const JOKER_SPRONE: u8 = 0;
/// Jolly "Scudo di brina": il Gelo non colpisce la fila di chi lo possiede.
pub const JOKER_SCUDO: u8 = 1;
/// Bit alto di uno slot del tabellone: proprietario (0 o 1).
const OWNER: u8 = 0x80;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pile<const N: usize> {
    pub c: [u8; N],
    pub n: u8,
}

impl<const N: usize> Pile<N> {
    pub const fn new() -> Self {
        Self { c: [0; N], n: 0 }
    }
    pub fn push(&mut self, v: u8) -> bool {
        if self.n as usize >= N {
            return false;
        }
        self.c[self.n as usize] = v;
        self.n += 1;
        true
    }
    /// Toglie la carta in posizione `i` (ordine conservato). `i` deve essere `< n`.
    pub fn take(&mut self, i: usize) -> u8 {
        let v = self.c[i];
        self.c.copy_within(i + 1..self.n as usize, i);
        self.n -= 1;
        v
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.c[..self.n as usize]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Side {
    pub deck: Pile<DECK>,
    pub hand: Pile<HAND>,
    pub grave: Pile<DECK>,
    /// File: 0 = vicina, 1 = a distanza. Contengono carte di entrambi i giocatori (vedi `OWNER`).
    pub rows: [Pile<ROW>; 2],
    pub wins: u8,
    pub passed: bool,
    /// Jolly attivi per questo giocatore (vedi `JOKER_*`).
    pub jokers: [Option<u8>; 2],
}

impl Side {
    const fn new() -> Self {
        Self { deck: Pile::new(), hand: Pile::new(), grave: Pile::new(), rows: [Pile::new(), Pile::new()], wins: 0, passed: false, jokers: [None; 2] }
    }
}

/// Esito di un round: punti di ciascun tabellone e vincitore (0 tu, 1 avversario).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Round {
    pub pts: [i32; 2],
    pub winner: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Game {
    /// 0 = giocatore, 1 = avversario.
    pub side: [Side; 2],
    pub turn: u8,
    /// Chi apre il round (ha perso il round precedente).
    pub starter: u8,
    pub frost: bool,
    pub fog: bool,
    /// Numero del round in corso, da 1.
    pub round: u16,
    /// Vincitore della partita, quando c'è.
    pub over: Option<u8>,
    /// Ultima carta piazzata sul tabellone: (tabellone, fila, slot). Serve per l'effetto visivo.
    pub last: Option<(u8, u8, u8)>,
    /// Esito dell'ultimo round concluso.
    pub result: Option<Round>,
    rng: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Act {
    /// Gioca la carta in posizione `i` della mano.
    Play(usize),
    Pass,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    Continue,
    Round(Round),
    MatchWon(u8),
}

fn row_idx(r: Row) -> usize {
    match r {
        Row::Close => 0,
        Row::Ranged => 1,
    }
}

impl Game {
    /// Nuova partita con i mazzi di default.
    pub fn new(seed: u32) -> Self {
        Self::with_decks(seed, &DECK_A, &DECK_B)
    }

    /// Nuova partita con mazzi scelti: `a` per il giocatore, `b` per l'avversario.
    pub fn with_decks(seed: u32, a: &[u8], b: &[u8]) -> Self {
        let mut g = Game {
            side: [Side::new(); 2],
            turn: 0,
            starter: 0,
            frost: false,
            fog: false,
            round: 1,
            over: None,
            last: None,
            result: None,
            rng: seed | 1,
        };
        for (s, deck) in [a, b].iter().enumerate() {
            for &id in deck.iter() {
                g.side[s].deck.push(id);
            }
            g.shuffle(s);
        }
        for _ in 0..HAND_START {
            for s in 0..2 {
                g.draw(s);
            }
        }
        g.starter = (g.rand() & 1) as u8;
        g.turn = g.starter;
        g
    }

    fn rand(&mut self) -> u32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        x
    }

    fn shuffle(&mut self, s: usize) {
        let n = self.side[s].deck.n as usize;
        for i in (1..n).rev() {
            let j = self.rand() as usize % (i + 1);
            self.side[s].deck.c.swap(i, j);
        }
    }

    fn reshuffle(&mut self, s: usize) {
        let grave = self.side[s].grave;
        self.side[s].grave.n = 0;
        for &id in grave.as_slice() {
            self.side[s].deck.push(id);
        }
        self.shuffle(s);
    }

    /// Pesca la carta in cima al mazzo; se il mazzo è vuoto rimescola il cimitero.
    pub fn draw(&mut self, s: usize) {
        if self.side[s].deck.n == 0 {
            self.reshuffle(s);
        }
        let n = self.side[s].deck.n as usize;
        if n == 0 {
            return;
        }
        let id = self.side[s].deck.take(n - 1);
        if (self.side[s].hand.n as usize) < HAND_CAP {
            self.side[s].hand.push(id);
        } else {
            self.side[s].grave.push(id);
        }
    }

    /// Schieramento: porta in mano tutte le carte del gruppo rimaste nel mazzo.
    fn muster(&mut self, s: usize, group: u8) {
        let mut i = 0;
        while i < self.side[s].deck.n as usize {
            let id = self.side[s].deck.c[i];
            if DEFS[id as usize].group == group && (self.side[s].hand.n as usize) < HAND_CAP {
                let id = self.side[s].deck.take(i);
                self.side[s].hand.push(id);
            } else {
                i += 1;
            }
        }
    }

    /// Guaritore: riporta in mano l'unità più forte del cimitero.
    fn revive(&mut self, s: usize) {
        if self.side[s].hand.n as usize >= HAND_CAP {
            return;
        }
        let grave = self.side[s].grave;
        let mut best: Option<(usize, u8)> = None;
        for (i, &id) in grave.as_slice().iter().enumerate() {
            let d = DEFS[id as usize];
            if d.kind == Kind::Unit && best.map_or(true, |(_, p)| d.power > p) {
                best = Some((i, d.power));
            }
        }
        if let Some((i, _)) = best {
            let id = self.side[s].grave.take(i);
            self.side[s].hand.push(id);
        }
    }

    /// Scorch: distrugge l'unità più forte del tabellone (a parità, la prima trovata).
    fn scorch(&mut self) {
        let mut best: Option<(usize, usize, usize, i32)> = None;
        for b in 0..2 {
            for r in 0..2 {
                for i in 0..self.side[b].rows[r].n as usize {
                    let slot = self.side[b].rows[r].c[i];
                    if DEFS[(slot & !OWNER) as usize].kind != Kind::Unit {
                        continue;
                    }
                    let p = self.slot_power(b, r, i);
                    if best.map_or(true, |x| p > x.3) {
                        best = Some((b, r, i, p));
                    }
                }
            }
        }
        if let Some((b, r, i, _)) = best {
            let slot = self.side[b].rows[r].take(i);
            self.side[(slot >> 7) as usize].grave.push(slot & !OWNER);
        }
    }

    /// Gioca la carta `idx` della mano di `s`, senza controlli di turno: usare `act`.
    pub(crate) fn play(&mut self, s: usize, idx: usize) {
        let id = self.side[s].hand.take(idx);
        let d = DEFS[id as usize];
        match d.kind {
            Kind::Frost => {
                self.frost = true;
                self.side[s].grave.push(id);
            }
            Kind::Fog => {
                self.fog = true;
                self.side[s].grave.push(id);
            }
            Kind::Clear => {
                self.frost = false;
                self.fog = false;
                self.side[s].grave.push(id);
            }
            Kind::Scorch => {
                self.side[s].grave.push(id);
                self.scorch();
            }
            _ => {
                let board = if d.kind == Kind::Spy { 1 - s } else { s };
                let slot = id | ((s as u8) << 7);
                let row = row_idx(d.row);
                self.side[board].rows[row].push(slot);
                self.last = Some((board as u8, row as u8, slot));
                if d.kind == Kind::Medic {
                    self.revive(s);
                }
                if d.group != 0 {
                    self.muster(s, d.group);
                }
                if d.kind == Kind::Spy {
                    self.draw(s);
                    self.draw(s);
                }
            }
        }
    }

    /// Potenza di uno slot del tabellone `board`, fila `row`, posizione `i`.
    /// Unità: base (ridotta a 1 dal meteo), +1 per ogni Stendardo/Trombettiere della fila, x2 con il Corno.
    pub fn slot_power(&self, board: usize, row: usize, i: usize) -> i32 {
        let rp = &self.side[board].rows[row];
        let d = DEFS[(rp.c[i] & !OWNER) as usize];
        let shielded = self.has_joker(board, JOKER_SCUDO);
        let affected = (row == 0 && self.frost && !shielded) || (row == 1 && self.fog);
        let mut p = d.power as i32;
        if affected && d.kind == Kind::Unit && p > 1 {
            p = 1;
        }
        if d.kind == Kind::Unit {
            let morale = rp.as_slice().iter().filter(|&&x| DEFS[(x & !OWNER) as usize].kind == Kind::Morale).count() as i32;
            p += morale;
            if rp.as_slice().iter().any(|&x| DEFS[(x & !OWNER) as usize].kind == Kind::Horn) {
                p *= 2;
            }
        }
        p
    }

    /// Il giocatore `who` ha il jolly `j` attivo?
    pub fn has_joker(&self, who: usize, j: u8) -> bool {
        self.side[who].jokers.contains(&Some(j))
    }

    /// Imposta i jolly del giocatore `who` (massimo due).
    pub fn set_jokers(&mut self, who: usize, j: [Option<u8>; 2]) {
        self.side[who].jokers = j;
    }

    /// Punteggio di un tabellone (chip × mult per fila, sommati). Chip = potenza delle carte della fila.
    /// Mult = 1, +1 con il jolly Sprone se la fila ha almeno 3 carte.
    pub fn score(&self, board: usize) -> i32 {
        let mut t = 0;
        for r in 0..2 {
            let n = self.side[board].rows[r].n as usize;
            let mut chips = 0;
            for i in 0..n {
                chips += self.slot_power(board, r, i);
            }
            let mult = if self.has_joker(board, JOKER_SPRONE) && n >= 3 { 2 } else { 1 };
            t += chips * mult;
        }
        t
    }

    /// Mossa del giocatore `who`. `None` se non è il suo turno, la carta non c'è o la partita è finita.
    pub fn act(&mut self, who: u8, a: Act) -> Option<Event> {
        if self.over.is_some() || who != self.turn {
            return None;
        }
        let s = who as usize;
        match a {
            Act::Play(i) => {
                if i >= self.side[s].hand.n as usize {
                    return None;
                }
                self.play(s, i);
            }
            Act::Pass => self.side[s].passed = true,
        }
        self.set_turn(1 - s);
        if self.side[0].passed && self.side[1].passed {
            return Some(self.end_round());
        }
        Some(Event::Continue)
    }

    /// Passa il turno a `next`, o al giocatore che può ancora agire. Chi non ha carte passa da solo.
    fn set_turn(&mut self, mut next: usize) {
        for _ in 0..2 {
            if !self.side[next].passed && self.side[next].hand.n > 0 {
                self.turn = next as u8;
                return;
            }
            self.side[next].passed = true;
            next = 1 - next;
        }
    }

    fn end_round(&mut self) -> Event {
        let pts = [self.score(0), self.score(1)];
        for b in 0..2 {
            for r in 0..2 {
                let row = self.side[b].rows[r];
                for &slot in row.as_slice() {
                    self.side[(slot >> 7) as usize].grave.push(slot & !OWNER);
                }
                self.side[b].rows[r].n = 0;
            }
        }
        self.frost = false;
        self.fog = false;
        self.last = None;
        self.round = self.round.saturating_add(1);
        for sd in self.side.iter_mut() {
            sd.passed = false;
        }
        // Pareggio: vince chi non apre. Così nessun round si ripete all'infinito.
        let starter = self.starter as usize;
        let w = if pts[0] > pts[1] {
            0
        } else if pts[1] > pts[0] {
            1
        } else {
            1 - starter
        };
        let round = Round { pts, winner: w as u8 };
        self.result = Some(round);
        self.side[w].wins += 1;
        if self.side[w].wins >= WINS {
            self.over = Some(w as u8);
            return Event::MatchWon(w as u8);
        }
        self.starter = (1 - w) as u8;
        for s in 0..2 {
            for _ in 0..DRAW_ROUND {
                self.draw(s);
            }
        }
        self.set_turn(self.starter as usize);
        Event::Round(round)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> Game {
        Game { side: [Side::new(); 2], turn: 0, starter: 0, frost: false, fog: false, round: 1, over: None, last: None, result: None, rng: 7 }
    }

    fn put(g: &mut Game, board: usize, row: usize, id: u8, owner: u8) {
        g.side[board].rows[row].push(id | (owner << 7));
    }

    #[test]
    fn mazzi_validi() {
        assert!(DECK_A.iter().chain(DECK_B.iter()).all(|&id| (id as usize) < DEFS.len()));
        assert_eq!(DECK_A.len(), DECK);
        assert_eq!(DECK_B.len(), DECK);
    }

    #[test]
    fn stendardo_da_plus_uno_alle_unita_non_a_se_stesso() {
        let mut g = empty();
        put(&mut g, 0, 0, 2, 0); // Guerriero 6
        put(&mut g, 0, 0, 9, 0); // Stendardo 2
        assert_eq!(g.score(0), 6 + 1 + 2);
    }

    #[test]
    fn stendardi_non_si_potenziano_a_vicenda() {
        let mut g = empty();
        put(&mut g, 0, 1, 17, 0); // Trombettiere 2
        put(&mut g, 0, 1, 17, 0); // Trombettiere 2
        assert_eq!(g.score(0), 4);
    }

    #[test]
    fn corno_raddoppia_le_unita_della_fila() {
        let mut g = empty();
        put(&mut g, 0, 0, 4, 0); // Gigante 10
        put(&mut g, 0, 0, 23, 0); // Corno 0
        assert_eq!(g.score(0), 20);
    }

    #[test]
    fn gelo_riduce_la_fila_vicina() {
        let mut g = empty();
        put(&mut g, 0, 0, 4, 0); // Gigante 10
        put(&mut g, 0, 1, 15, 0); // Catapulta 8: a distanza, non toccata
        g.frost = true;
        assert_eq!(g.score(0), 1 + 8);
    }

    #[test]
    fn scorch_distrugge_l_unita_piu_forte() {
        let mut g = empty();
        put(&mut g, 0, 0, 4, 0); // Gigante 10 (del giocatore)
        put(&mut g, 1, 0, 2, 1); // Guerriero 6 (avversario)
        g.side[0].hand.push(24); // Scorch
        g.play(0, 0);
        assert_eq!(g.side[0].rows[0].n, 0);
        assert_eq!(g.side[1].rows[0].n, 1);
        assert!(g.side[0].grave.as_slice().contains(&4));
    }

    #[test]
    fn spia_finisce_nella_fila_avversaria_e_pesca_due() {
        let mut g = empty();
        g.side[0].deck.push(0);
        g.side[0].deck.push(0);
        g.side[0].hand.push(10); // Spia
        g.play(0, 0);
        assert_eq!(g.side[1].rows[0].n, 1);
        assert_eq!(g.side[1].rows[0].c[0] & OWNER, 0);
        assert_eq!(g.side[0].hand.n, 2);
    }

    #[test]
    fn schieramento_pesca_il_gruppo() {
        let mut g = empty();
        g.side[0].deck.push(5);
        g.side[0].deck.push(6);
        g.side[0].deck.push(0);
        g.side[0].hand.push(7);
        g.play(0, 0);
        assert_eq!(g.side[0].hand.as_slice(), &[5, 6]);
        assert_eq!(g.side[0].deck.as_slice(), &[0]);
    }

    #[test]
    fn pass_chiude_il_round_e_vince_chi_ha_piu_potenza() {
        let mut g = Game::new(42);
        g.side[0].hand = Pile::new();
        g.side[0].hand.push(4); // Gigante 10
        g.side[1].hand = Pile::new();
        g.side[1].hand.push(0); // Fante 2
        g.turn = 0;
        assert_eq!(g.act(0, Act::Play(0)), Some(Event::Continue));
        assert_eq!(g.turn, 1);
        assert!(matches!(g.act(1, Act::Pass), Some(Event::Round(r)) if r.winner == 0));
        assert_eq!(g.side[0].wins, 1);
        assert_eq!(g.starter, 1); // chi perde il round apre il successivo
        assert_eq!(g.round, 2);
        assert_eq!(g.turn, 1);
    }

    #[test]
    fn pareggio_lo_vince_chi_non_apre() {
        let mut g = empty();
        for s in 0..2 {
            g.side[s].rows[0].push(2); // Guerriero 6 su entrambi
            g.side[s].deck.push(0);
            g.side[s].deck.push(0);
        }
        g.side[1].passed = true;
        g.turn = 0; // apre il giocatore 0
        assert!(matches!(g.act(0, Act::Pass), Some(Event::Round(r)) if r.winner == 1));
        assert_eq!(g.side[1].wins, 1);
        assert_eq!(g.starter, 0); // ha perso il round, quindi apre di nuovo
    }

    #[test]
    fn vittoria_di_partita_al_secondo_round() {
        let mut g = empty();
        g.side[0].wins = 1;
        g.side[0].hand.push(4); // Gigante 10
        g.turn = 0;
        assert_eq!(g.act(0, Act::Play(0)), Some(Event::MatchWon(0)));
        assert_eq!(g.over, Some(0));
        assert_eq!(g.act(0, Act::Pass), None); // partita finita: nessuna mossa
    }

    #[test]
    fn partite_complete_terminano() {
        for seed in 1..300u32 {
            let mut g = Game::new(seed);
            let mut n = 0;
            while g.over.is_none() {
                let who = g.turn as usize;
                let a = ai::choose(&g, who);
                assert!(g.act(who as u8, a).is_some());
                n += 1;
                assert!(n < 5000, "partita {seed} non termina");
            }
            assert!(g.side[g.over.unwrap() as usize].wins >= WINS);
        }
    }
}

#[cfg(test)]
mod joker_tests {
    use super::*;

    #[test]
    fn senza_jolly_il_punteggio_non_cambia() {
        let mut g = Game::new(3);
        g.set_jokers(0, [None, None]);
        let base: i32 = (0..2).map(|r| (0..g.side[0].rows[r].n as usize).map(|i| g.slot_power(0, r, i)).sum::<i32>()).sum();
        assert_eq!(g.score(0), base);
    }

    #[test]
    fn sprone_raddoppia_la_fila_con_tre_carte() {
        let mut g = Game::new(3);
        g.frost = false;
        g.fog = false;
        g.side[0].rows[0] = Pile::new();
        g.side[0].rows[1] = Pile::new();
        for _ in 0..3 {
            g.side[0].rows[0].push(0);
        }
        let chips = (0..3).map(|i| g.slot_power(0, 0, i)).sum::<i32>();
        g.set_jokers(0, [Some(JOKER_SPRONE), None]);
        assert_eq!(g.score(0), chips * 2);
    }

    #[test]
    fn scudo_protegge_la_propria_fila_dal_gelo() {
        let mut g = Game::new(3);
        g.side[0].rows[0] = Pile::new();
        g.side[0].rows[0].push(4); // Gigante
        g.side[0].rows[1] = Pile::new();
        g.frost = false;
        let pieno = g.slot_power(0, 0, 0);
        g.frost = true;
        g.set_jokers(0, [None, None]);
        assert_eq!(g.slot_power(0, 0, 0), 1);
        g.set_jokers(0, [Some(JOKER_SCUDO), None]);
        assert_eq!(g.slot_power(0, 0, 0), pieno);
    }

    #[test]
    fn stendardo_sotto_gelo_resta_due() {
        let mut g = Game::new(3);
        g.side[0].rows[0] = Pile::new();
        g.side[0].rows[0].push(9); // Stendardo, potenza 2
        g.frost = true;
        assert_eq!(g.slot_power(0, 0, 0), 2);
    }
}
