//! Avversario: sceglie la mossa che aumenta di più il vantaggio sul tabellone; altrimenti passa.

use crate::cards::{Kind, DEFS};
use crate::{Act, Game};

pub fn choose(g: &Game, who: usize) -> Act {
    let opp = 1 - who;
    let cur = g.score(who) - g.score(opp);
    if g.side[opp].passed && cur > 0 {
        return Act::Pass;
    }
    let mut best = (cur, Act::Pass);
    for i in 0..g.side[who].hand.n as usize {
        let d = DEFS[g.side[who].hand.c[i] as usize];
        let mut h = *g;
        h.play(who, i);
        let mut v = h.score(who) - h.score(opp);
        // Carte che pescano (spia, guaritore, schieramento) valgono qualche punto in più.
        if matches!(d.kind, Kind::Spy | Kind::Medic) || d.group != 0 {
            v += 3;
        }
        if v > best.0 {
            best = (v, Act::Play(i));
        }
    }
    best.1
}
