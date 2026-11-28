//! IA: guida verso un punto poco più avanti sul centro pista; frena nelle curve.

use crate::{
    fx::{dir, mul, ONE},
    kart::{Input, Kart, MAX_V},
    track::{Near, Track},
};

pub fn drive(track: &Track, k: &Kart, near: &Near) -> Input {
    let look = if k.v > MAX_V / 2 { 3 } else { 2 };
    let (tx, ty) = track.point(near.seg + look, ONE / 2);
    let (dx, dy) = dir(k.h);
    let (vx, vy) = (tx - k.x, ty - k.y);
    let cross = mul(dx, vy) - mul(dy, vx);
    let dot = mul(dx, vx) + mul(dy, vy);
    let steer: i8 = if dot < 0 {
        if cross >= 0 { 1 } else { -1 }
    } else if cross > 12 * ONE {
        1
    } else if cross < -12 * ONE {
        -1
    } else {
        0
    };
    Input { gas: !(steer != 0 && k.v > MAX_V * 3 / 4), brake: false, steer, item: false }
}
