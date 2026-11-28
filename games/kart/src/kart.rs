//! Fisica del kart. Unità: mondo per tick; direzione in passi di angolo (256 = giro).

use crate::fx::{dir, div, mul, Fx, FULL, ONE};

/// Velocità massima, unità di mondo per tick.
pub const MAX_V: Fx = 4 * ONE;
const ACC: Fx = ONE / 16;
const BRAKE: Fx = ONE / 8;
const DRAG: Fx = ONE / 64;
const OFF_DRAG: Fx = ONE / 32;
/// Rotazione massima per tick (passi di angolo) a piena sterzata e velocità massima.
const TURN: Fx = ONE * 3 / 2;
pub const SPIN_TICKS: u16 = 60;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Item {
    #[default]
    None,
    Mushroom,
    Banana,
    Shell,
    Star,
}

/// Comandi di un tick. `steer`: -1 sinistra, 0 dritto, 1 destra.
#[derive(Clone, Copy, Default)]
pub struct Input {
    pub gas: bool,
    pub brake: bool,
    pub steer: i8,
    pub item: bool,
}

#[derive(Clone, Copy, Default)]
pub struct Kart {
    pub x: Fx,
    pub y: Fx,
    pub h: Fx,
    pub v: Fx,
    pub spin: u16,
    pub boost: u16,
    pub star: u16,
    pub item: Item,
    /// Ultimo segmento visto, per il conteggio dei giri.
    pub seg: usize,
    pub t: Fx,
    /// Giri completati.
    pub count: u8,
    /// Tick del traguardo finale.
    pub finish: Option<u32>,
}

fn decay(v: Fx, d: Fx) -> Fx {
    if v > d {
        v - d
    } else {
        0
    }
}

impl Kart {
    /// Un tick di fisica. `offroad`: il kart è sull'erba.
    pub fn drive(&mut self, inp: Input, offroad: bool) {
        let mut inp = inp;
        if self.spin > 0 {
            self.spin -= 1;
            self.h += 8 * ONE;
            inp = Input::default();
        }
        let fast = self.boost > 0 || self.star > 0;
        let mut vmax = if fast { MAX_V * 3 / 2 } else { MAX_V };
        if offroad {
            vmax /= 2;
        }
        if inp.gas {
            self.v += if fast { 2 * ACC } else { ACC };
        } else {
            self.v = decay(self.v, DRAG);
        }
        if inp.brake {
            self.v = decay(self.v, BRAKE);
        }
        if offroad {
            self.v = decay(self.v, OFF_DRAG);
        }
        self.v = self.v.clamp(0, vmax);
        // Lo sterzo è pieno solo a velocità piena: da fermo non ruota.
        let ratio = div(self.v, MAX_V);
        self.h = (self.h + mul(TURN * inp.steer as Fx, ratio)).rem_euclid(FULL);
        let (dx, dy) = dir(self.h);
        self.x += mul(dx, self.v);
        self.y += mul(dy, self.v);
        self.boost = self.boost.saturating_sub(1);
        self.star = self.star.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAS: Input = Input { gas: true, brake: false, steer: 0, item: false };

    fn run(k: &mut Kart, inp: Input, ticks: usize, off: bool) {
        for _ in 0..ticks {
            k.drive(inp, off);
        }
    }

    #[test]
    fn accelerates_to_max() {
        let mut k = Kart::default();
        run(&mut k, GAS, 300, false);
        assert_eq!(k.v, MAX_V);
    }

    #[test]
    fn coasts_to_stop() {
        let mut k = Kart { v: MAX_V, ..Default::default() };
        run(&mut k, Input::default(), 300, false);
        assert_eq!(k.v, 0);
    }

    #[test]
    fn no_turn_when_stopped() {
        let mut k = Kart::default();
        run(&mut k, Input { steer: 1, ..Default::default() }, 50, false);
        assert_eq!(k.h, 0);
    }

    #[test]
    fn turns_at_speed() {
        let mut k = Kart { v: MAX_V, ..Default::default() };
        run(&mut k, Input { gas: true, steer: 1, ..Default::default() }, 10, false);
        assert!(k.h > 0 && k.h < FULL / 4);
    }

    #[test]
    fn straight_motion_follows_heading() {
        let mut k = Kart { v: MAX_V, ..Default::default() };
        k.drive(Input::default(), false);
        assert!(k.x > 0);
        assert_eq!(k.y, 0);
    }

    #[test]
    fn offroad_halves_top_speed() {
        let mut k = Kart::default();
        run(&mut k, GAS, 300, true);
        assert!(k.v <= MAX_V / 2);
    }

    #[test]
    fn boost_raises_top_speed() {
        let mut k = Kart { boost: 300, ..Default::default() };
        run(&mut k, GAS, 300, false);
        assert!(k.v > MAX_V);
    }
}
