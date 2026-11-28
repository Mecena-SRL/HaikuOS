//! Pilota automatico per demo e test: corre a destra, salta su muri, buche e Fungo.
//! Se resta incastrato (velocità quasi nulla) arretra per qualche frame.
use crate::*;

#[derive(Default)]
pub struct Bot {
    hold: u32,
    stuck: u32,
    back: u32,
    toggle: bool,
}

impl Bot {
    pub fn input(&mut self, g: &Game) -> u8 {
        match g.state {
            State::Playing => {}
            State::Clear => return 0,
            State::Map | State::Over | State::Won => {
                self.toggle = !self.toggle;
                return if self.toggle { BTN_START } else { 0 };
            }
        }
        let p = &g.p;
        let t = TILE * SUB;
        let feet = (p.b.y + p.b.h * SUB).div_euclid(t);
        let ahead = (p.b.x + (p.b.w + 4) * SUB).div_euclid(t);
        let wall = g.lvl.solid(ahead, feet - 1);
        let pit = !g.lvl.solid(ahead, feet);
        let fungo = g.mobs.iter().any(|m| {
            let d = m.b.x - p.b.x;
            m.alive && m.kind == MobKind::Fungo && d > 4 * SUB && d < 32 * SUB
        });
        let pit_soon = (1..=4).any(|k| !g.lvl.solid(ahead + k, feet));
        if p.on_ground && self.hold < 22 {
            self.hold = 0; // atterrato: rilascia il tasto salto
        }
        if p.on_ground && (wall || pit || (fungo && !pit_soon)) && self.hold == 0 {
            self.hold = 22;
        }
        self.stuck = if p.b.vx.abs() < 8 { self.stuck + 1 } else { 0 };
        if self.stuck > 30 {
            self.stuck = 0;
            self.back = 20;
        }
        if self.back > 0 {
            self.back -= 1;
            return BTN_LEFT;
        }
        let mut bits = BTN_RIGHT | BTN_RUN;
        if self.hold > 0 {
            bits |= BTN_JUMP;
            self.hold -= 1;
        }
        bits
    }
}
