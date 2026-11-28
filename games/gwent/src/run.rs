//! Run stile Balatro: una sequenza di sfide contro l'avversario, con denaro e negozio di jolly tra una sfida e l'altra.
//! Logica pura, `no_std`, senza allocazione.

use crate::{Game, JOKER_SCUDO, JOKER_SPRONE};

/// Sfide da vincere per completare la run.
pub const STAGES: u8 = 4;
/// Denaro guadagnato a ogni sfida vinta.
pub const WIN_MONEY: u8 = 3;
/// Costo di un jolly nel negozio.
pub const JOKER_COST: u8 = 2;
/// Jolly acquistabili (ids di `JOKER_*`).
pub const POOL: [u8; 2] = [JOKER_SPRONE, JOKER_SCUDO];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Run {
    /// Sfida in corso, da 0 a STAGES-1.
    pub stage: u8,
    pub money: u8,
    /// Jolly posseduti (massimo due).
    pub jokers: [Option<u8>; 2],
    /// Jolly in vendita nel negozio.
    pub shop: [Option<u8>; 2],
    rng: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stato {
    /// Sfida successiva da giocare.
    Prossima,
    /// Run completata: tutte le sfide vinte.
    Completata,
}

impl Run {
    pub fn new(seed: u32) -> Self {
        let mut r = Run { stage: 0, money: 0, jokers: [None; 2], shop: [None; 2], rng: seed | 1 };
        r.riempi_negozio();
        r
    }

    fn next(&mut self) -> u32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        x
    }

    fn riempi_negozio(&mut self) {
        for i in 0..self.shop.len() {
            let k = (self.next() % POOL.len() as u32) as usize;
            self.shop[i] = Some(POOL[k]);
        }
    }

    /// Nuovo giro di negozio (costa 1 denaro).
    pub fn riroll(&mut self) -> bool {
        if self.money < 1 {
            return false;
        }
        self.money -= 1;
        self.riempi_negozio();
        true
    }

    /// Compra il jolly in vendita in posizione `slot`. Richiede denaro e uno slot libero.
    pub fn compra(&mut self, slot: usize) -> bool {
        if slot >= self.shop.len() || self.money < JOKER_COST {
            return false;
        }
        let Some(j) = self.shop[slot] else { return false };
        let Some(libero) = self.jokers.iter().position(|x| x.is_none()) else { return false };
        self.money -= JOKER_COST;
        self.jokers[libero] = Some(j);
        self.shop[slot] = None;
        true
    }

    /// Chiude la sfida vinta: denaro, sfida successiva e negozio nuovo.
    pub fn vinci_sfida(&mut self) -> Stato {
        self.money = self.money.saturating_add(WIN_MONEY);
        self.stage += 1;
        if self.stage >= STAGES {
            return Stato::Completata;
        }
        self.riempi_negozio();
        Stato::Prossima
    }

    /// Imposta sul tabellone del giocatore i jolly della run.
    pub fn applica(&self, g: &mut Game) {
        g.set_jokers(0, self.jokers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compra_scala_il_denaro_e_riempie_uno_slot() {
        let mut r = Run::new(5);
        r.money = 2;
        assert!(r.compra(0));
        assert_eq!(r.money, 0);
        assert!(r.jokers[0].is_some());
        assert!(!r.compra(1), "senza denaro non si compra");
    }

    #[test]
    fn niente_acquisto_con_slot_pieni() {
        let mut r = Run::new(5);
        r.money = 10;
        assert!(r.compra(0));
        assert!(r.compra(1));
        r.riempi_negozio();
        assert!(!r.compra(0), "massimo due jolly");
    }

    #[test]
    fn vincere_quattro_sfide_completa_la_run() {
        let mut r = Run::new(9);
        for _ in 0..STAGES - 1 {
            assert_eq!(r.vinci_sfida(), Stato::Prossima);
        }
        assert_eq!(r.vinci_sfida(), Stato::Completata);
        assert_eq!(r.money, WIN_MONEY * STAGES);
    }

    #[test]
    fn i_jolly_arrivano_sul_tabellone() {
        let mut r = Run::new(4);
        r.money = 2;
        r.compra(0);
        let mut g = Game::new(3);
        r.applica(&mut g);
        assert_eq!(g.side[0].jokers, r.jokers);
    }
}
