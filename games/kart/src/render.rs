//! Disegno su buffer ARGB u32. Camera centrata sul kart `cam`; 1 pixel = 2 unità di mondo.

use crate::{
    fx::{Fx, ONE},
    race::{Haz, Race, N_BOXES},
};

const GRASS: u32 = 0xFF2E7D32;
const ROAD: u32 = 0xFF616161;
const BOX: u32 = 0xFFFFD600;
const BANANA: u32 = 0xFFFFEB3B;
const SHELL: u32 = 0xFFE53935;
const COLORS: [u32; 8] = [
    0xFFFF1744, 0xFF2979FF, 0xFF00E676, 0xFFFF9100, 0xFFD500F9, 0xFF00E5FF, 0xFFFF80AB, 0xFFEEEEEE,
];

fn rect(buf: &mut [u32], w: usize, h: usize, x: i32, y: i32, rw: i32, rh: i32, c: u32) {
    let (x0, y0) = (x.max(0), y.max(0));
    let (x1, y1) = ((x + rw).min(w as i32), (y + rh).min(h as i32));
    for py in y0..y1 {
        let row = py as usize * w;
        for px in x0..x1 {
            buf[row + px as usize] = c;
        }
    }
}

pub fn draw(race: &Race, buf: &mut [u32], w: usize, h: usize, cam: usize) {
    buf.fill(GRASS);
    let c = race.karts[cam];
    let (ox, oy) = (w as i32 / 2, h as i32 / 2);
    let sx = |x: Fx| ox + ((x - c.x) >> 17);
    let sy = |y: Fx| oy + ((y - c.y) >> 17);
    let tr = race.track;
    let r = tr.half_w / 2;
    for seg in 0..tr.n() {
        for s in 0..=8 {
            let (x, y) = tr.point(seg, s * ONE / 8);
            rect(buf, w, h, sx(x) - r, sy(y) - r, 2 * r, 2 * r, ROAD);
        }
    }
    let n = tr.n();
    for b in 0..N_BOXES {
        if race.boxes[b] == 0 {
            let (x, y) = tr.point(b * n / N_BOXES, ONE / 2);
            rect(buf, w, h, sx(x) - 4, sy(y) - 4, 8, 8, BOX);
        }
    }
    for hz in race.haz.iter().filter(|z| z.kind != Haz::None) {
        let col = if hz.kind == Haz::Banana { BANANA } else { SHELL };
        rect(buf, w, h, sx(hz.x) - 3, sy(hz.y) - 3, 6, 6, col);
    }
    for i in 0..race.n {
        let k = race.karts[i];
        rect(buf, w, h, sx(k.x) - 4, sy(k.y) - 4, 8, 8, COLORS[i]);
    }
}
