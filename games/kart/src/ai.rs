//! IA: segue la linea centrale con un punto obiettivo a distanza d'arco fissa davanti al kart.
//! Sterzo proporzionale all'errore angolare; frena in curva quando l'errore è grande.

use crate::{
    fx::{dir, mul, Fx, ONE},
    kart::{Input, Kart, MAX_V, STEER_FULL},
    track::{Near, Track},
};

/// Distanza d'arco dell'obiettivo davanti al kart (unità di mondo).
const LOOK: Fx = 60 * ONE;

/// Punto della linea centrale a distanza d'arco `dist` dalla proiezione del kart.
fn lookahead(track: &Track, near: &Near, dist: Fx) -> (Fx, Fx) {
    let total = track.n() as i64 * ONE as i64;
    let step = (ONE / 16) as i64;
    let lim = dist as i64 * dist as i64;
    let mut g = near.seg as i64 * ONE as i64 + near.t as i64;
    loop {
        g += step;
        if g >= total {
            g -= total;
        }
        let (x, y) = track.point((g >> 16) as usize, (g & 0xFFFF) as Fx);
        let (ax, ay) = ((x - near.x) as i64, (y - near.y) as i64);
        if ax * ax + ay * ay >= lim {
            return (x, y);
        }
    }
}

pub fn drive(track: &Track, k: &Kart, near: &Near) -> Input {
    let (tx, ty) = lookahead(track, near, LOOK);
    let (dx, dy) = dir(k.h);
    let (vx, vy) = (tx - k.x, ty - k.y);
    let cross = mul(dx, vy) - mul(dy, vx);
    let dot = mul(dx, vx) + mul(dy, vy);
    // Errore angolare come tangente: pieno sterzo da 10 gradi in su, proporzionale sotto.
    let steer: i8 = if dot <= 0 {
        if cross >= 0 { STEER_FULL } else { -STEER_FULL }
    } else {
        let tan = crate::fx::div(cross, dot);
        ((tan as i64 * 23 >> 16).clamp(-STEER_FULL as i64, STEER_FULL as i64)) as i8
    };
    let err = steer.unsigned_abs() as i32;
    let brake = err >= STEER_FULL as i32 / 4 && k.v > MAX_V / 3;
    Input { gas: !brake, brake, steer, item: false }
}
