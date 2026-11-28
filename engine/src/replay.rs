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

/// Errori di registrazione e replay: nessuno viene ignorato in silenzio.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplayError {
    /// Il replay diverge dalla registrazione al tick indicato.
    Diverged(usize),
    /// Più corpi di `MAX_BODIES`.
    TooManyBodies,
    /// Il registro è troppo piccolo per tutti gli input.
    Truncated,
}

/// Numero massimo di corpi in una sessione registrata.
pub const MAX_BODIES: usize = 8;

/// Esegue `start` sugli input registrati e ritorna Err(Diverged(t)) al primo tick con hash diverso.
/// Lo stato di partenza deve essere lo stesso usato durante la registrazione.
pub fn replay<const N: usize>(rec: &Recorder<N>, start: &[Body], m: &Map) -> Result<(), ReplayError> {
    if start.len() > MAX_BODIES {
        return Err(ReplayError::TooManyBodies);
    }
    let mut bodies = [Body::default(); MAX_BODIES];
    let n = start.len();
    bodies[..n].copy_from_slice(start);
    for t in 0..rec.len {
        for b in bodies[..n].iter_mut() {
            tick(b, m, rec.inputs[t]);
        }
        if state_hash(&bodies[..n]) != rec.hashes[t] {
            return Err(ReplayError::Diverged(t));
        }
    }
    Ok(())
}

/// Registra una sessione: applica `inputs` a `start`, salva hash per tick.
pub fn record<const N: usize>(start: &[Body], m: &Map, inputs: &[Input]) -> Result<Recorder<N>, ReplayError> {
    if start.len() > MAX_BODIES {
        return Err(ReplayError::TooManyBodies);
    }
    let mut rec = Recorder::new();
    let mut bodies = [Body::default(); MAX_BODIES];
    let n = start.len();
    bodies[..n].copy_from_slice(start);
    for &inp in inputs {
        for b in bodies[..n].iter_mut() {
            tick(b, m, inp);
        }
        if !rec.push(inp, state_hash(&bodies[..n])) {
            return Err(ReplayError::Truncated);
        }
    }
    Ok(rec)
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
        let rec: Recorder<240> = record(&start(), &m, &inputs()).unwrap();
        assert_eq!(rec.len(), 240);
        assert_eq!(replay(&rec, &start(), &m), Ok(()));
    }

    #[test]
    fn detects_first_divergent_tick() {
        let cells = level();
        let m = Map { w: W, h: H, tile: 16, cells: &cells };
        let mut rec: Recorder<240> = record(&start(), &m, &inputs()).unwrap();
        // corrompe l'hash al tick 100: il primo divergente deve essere 100
        rec.hashes[100] ^= 1;
        assert_eq!(replay(&rec, &start(), &m), Err(ReplayError::Diverged(100)));
    }

    #[test]
    fn rejects_truncated_and_too_many_bodies() {
        let cells = level();
        let m = Map { w: W, h: H, tile: 16, cells: &cells };
        assert_eq!(record::<4>(&start(), &m, &inputs()).err(), Some(ReplayError::Truncated));
        let nine = [Body::default(); 9];
        assert_eq!(record::<240>(&nine, &m, &inputs()).err(), Some(ReplayError::TooManyBodies));
    }

    #[test]
    fn recorder_is_bounded() {
        let mut r: Recorder<2> = Recorder::new();
        assert!(r.push(0, 1) && r.push(0, 2));
        assert!(!r.push(0, 3));
        assert_eq!(r.len(), 2);
    }
}
