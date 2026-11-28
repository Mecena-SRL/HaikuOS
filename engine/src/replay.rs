//! Registrazione e replay deterministici: input per tick più hash di stato.
//! Buffer a capacità fissa `N`, nessuna allocazione. Il primo tick che diverge viene segnalato.

use crate::{state_hash, step, Body, Map, Fx, ONE};

/// Input di un tick: bit 0 sinistra, bit 1 destra, bit 2 salto.
pub type Input = u8;
pub const LEFT: Input = 1;
pub const RIGHT: Input = 2;
pub const JUMP: Input = 4;

/// Passo di simulazione in tick al secondo (fisso, non dipende dal tempo reale).
pub const TICKS_PER_SEC: u32 = 60;
const RUN: Fx = ONE / 2;
const JUMP_V: Fx = -3 * ONE;

/// Applica l'input a un corpo e avanza di un tick. È l'unico punto che modifica lo stato.
pub fn tick(b: &mut Body, m: &Map, input: Input) {
    b.vx = match (input & RIGHT != 0, input & LEFT != 0) {
        (true, false) => RUN,
        (false, true) => -RUN,
        _ => 0,
    };
    if input & JUMP != 0 && b.on_ground {
        b.vy = JUMP_V;
    }
    step(b, m);
}

/// Registro a capacità fissa: input e hash di stato dopo ogni tick.
pub struct Recorder<const N: usize> {
    inputs: [Input; N],
    hashes: [u32; N],
    len: usize,
}

impl<const N: usize> Default for Recorder<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Recorder<N> {
    pub const fn new() -> Self {
        Self { inputs: [0; N], hashes: [0; N], len: 0 }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// Ritorna false se il registro è pieno.
    pub fn push(&mut self, input: Input, hash: u32) -> bool {
        if self.len == N {
            return false;
        }
        self.inputs[self.len] = input;
        self.hashes[self.len] = hash;
        self.len += 1;
        true
    }
}

/// Esegue `bodies` sugli input registrati e ritorna il primo tick con hash diverso, o None.
/// Lo stato di partenza deve essere lo stesso usato durante la registrazione.
pub fn replay<const N: usize>(rec: &Recorder<N>, start: &[Body], m: &Map) -> Option<usize> {
    let mut bodies = [Body::default(); 8];
    let n = start.len().min(bodies.len());
    bodies[..n].copy_from_slice(&start[..n]);
    for t in 0..rec.len {
        for b in bodies[..n].iter_mut() {
            tick(b, m, rec.inputs[t]);
        }
        if state_hash(&bodies[..n]) != rec.hashes[t] {
            return Some(t);
        }
    }
    None
}

/// Registra una sessione: applica `inputs` a `start`, salva hash per tick.
pub fn record<const N: usize>(start: &[Body], m: &Map, inputs: &[Input]) -> Recorder<N> {
    let mut rec = Recorder::new();
    let mut bodies = [Body::default(); 8];
    let n = start.len().min(bodies.len());
    bodies[..n].copy_from_slice(&start[..n]);
    for &inp in inputs {
        for b in bodies[..n].iter_mut() {
            tick(b, m, inp);
        }
        if !rec.push(inp, state_hash(&bodies[..n])) {
            break;
        }
    }
    rec
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::px;

    const W: usize = 16;
    const H: usize = 8;
    fn level() -> [u8; W * H] {
        let mut c = [0u8; W * H];
        for x in 0..W {
            c[7 * W + x] = 1;
        }
        c
    }
    fn start() -> [Body; 2] {
        [
            Body { x: px(20), y: px(0), w: 6, h: 6, ..Default::default() },
            Body { x: px(60), y: px(10), w: 6, h: 6, ..Default::default() },
        ]
    }
    fn inputs() -> [Input; 240] {
        let mut v = [0u8; 240];
        for (i, x) in v.iter_mut().enumerate() {
            *x = match i / 40 {
                0 => RIGHT,
                1 => RIGHT | JUMP,
                2 => LEFT,
                3 => 0,
                4 => JUMP,
                _ => RIGHT,
            };
        }
        v
    }

    #[test]
    fn replay_matches_recording() {
        let cells = level();
        let m = Map { w: W, h: H, tile: 16, cells: &cells };
        let rec: Recorder<240> = record(&start(), &m, &inputs());
        assert_eq!(rec.len(), 240);
        assert_eq!(replay(&rec, &start(), &m), None);
    }

    #[test]
    fn detects_first_divergent_tick() {
        let cells = level();
        let m = Map { w: W, h: H, tile: 16, cells: &cells };
        let mut rec: Recorder<240> = record(&start(), &m, &inputs());
        // corrompe l'hash al tick 100: il primo divergente deve essere 100
        rec.hashes[100] ^= 1;
        assert_eq!(replay(&rec, &start(), &m), Some(100));
    }

    #[test]
    fn recorder_is_bounded() {
        let mut r: Recorder<2> = Recorder::new();
        assert!(r.push(0, 1) && r.push(0, 2));
        assert!(!r.push(0, 3));
        assert_eq!(r.len(), 2);
    }
}
