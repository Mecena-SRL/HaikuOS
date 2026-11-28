//! Rendering su framebuffer ARGB (VIEW_W x VIEW_H). Legge lo stato, non lo modifica.
use crate::levels::LEVELS;
use crate::*;

const SKY: u32 = 0xFF6BB8F0;
const GROUND: u32 = 0xFF8B5A2B;
const SEAM: u32 = 0xFF7A3020;
const BRICK: u32 = 0xFFB5523B;
const QUEST: u32 = 0xFFF2B233;
const USED: u32 = 0xFF6E5A4A;
const COIN: u32 = 0xFFFFD700;
const FLAG: u32 = 0xFF2E8B57;
const POLE: u32 = 0xFFEEEEEE;
const FUNGO: u32 = 0xFF9C5A2E;
const MUSH: u32 = 0xFFE03C31;
const STAR_C: u32 = 0xFFFFE94D;
const PLAYER: u32 = 0xFF2A5FD8;
const CAP: u32 = 0xFFD83A2A;
const WHITE: u32 = 0xFFFFFFFF;
const MAP_BG: u32 = 0xFF1F3A5F;

const FONT: [[u8; 5]; 10] = [
    [7, 5, 5, 5, 7],
    [2, 6, 2, 2, 7],
    [7, 1, 7, 4, 7],
    [7, 1, 7, 1, 7],
    [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7],
    [7, 4, 7, 5, 7],
    [7, 1, 2, 2, 2],
    [7, 5, 7, 5, 7],
    [7, 5, 7, 1, 7],
];

/// Disegna lo stato corrente di `g` in `fb` (VIEW_W * VIEW_H pixel).
pub fn draw(g: &Game, fb: &mut [u32]) {
    if g.state == State::Map {
        return draw_map(g, fb);
    }
    fill(fb, SKY);
    draw_level(g, fb);
    draw_hud(g, fb);
    match g.state {
        State::Clear => banner(fb, FLAG),
        State::Over => banner(fb, 0xFFB00020),
        State::Won => banner(fb, COIN),
        _ => {}
    }
}

fn fill(fb: &mut [u32], c: u32) {
    fb.iter_mut().for_each(|p| *p = c);
}

fn rect(fb: &mut [u32], x: i32, y: i32, w: i32, h: i32, c: u32) {
    let (x0, y0) = (x.max(0), y.max(0));
    let (x1, y1) = ((x + w).min(VIEW_W), (y + h).min(VIEW_H));
    for yy in y0..y1 {
        let row = (yy * VIEW_W) as usize;
        for xx in x0..x1 {
            fb[row + xx as usize] = c;
        }
    }
}

fn banner(fb: &mut [u32], c: u32) {
    rect(fb, 48, 100, 160, 40, c);
}

fn draw_level(g: &Game, fb: &mut [u32]) {
    let cam = g.cam_x;
    let lvl = &g.lvl;
    let tx0 = cam / TILE;
    let tx1 = ((cam + VIEW_W) / TILE + 1).min(lvl.w as i32);
    for ty in 0..lvl.h as i32 {
        for tx in tx0..tx1 {
            let (x, y) = (tx * TILE - cam, ty * TILE);
            match lvl.tile(tx, ty) {
                Tile::Ground => rect(fb, x, y, TILE, TILE, GROUND),
                Tile::Brick => {
                    rect(fb, x, y, TILE, TILE, BRICK);
                    rect(fb, x, y + 7, TILE, 2, SEAM);
                }
                Tile::Question(_) => {
                    rect(fb, x, y, TILE, TILE, QUEST);
                    rect(fb, x + 6, y + 4, 4, 8, 0xFF8A5A00);
                }
                Tile::Used => rect(fb, x, y, TILE, TILE, USED),
                Tile::Coin => rect(fb, x + 4, y + 2, 8, 12, COIN),
                Tile::Flag => {
                    rect(fb, x + 7, y, 2, TILE, POLE);
                    rect(fb, x + 9, y + 1, 7, 5, FLAG);
                }
                Tile::Air => {}
            }
        }
    }
    for m in g.mobs.iter().filter(|m| m.alive) {
        let (x, y) = (m.b.x / SUB - cam, m.b.y / SUB);
        match m.kind {
            MobKind::Fungo => {
                rect(fb, x, y + 2, 14, 12, FUNGO);
                rect(fb, x + 3, y + 5, 2, 2, WHITE);
                rect(fb, x + 9, y + 5, 2, 2, WHITE);
            }
            MobKind::Mushroom => {
                rect(fb, x, y, 14, 14, MUSH);
                rect(fb, x + 3, y + 8, 8, 4, WHITE);
            }
            MobKind::Star => rect(fb, x + 2, y + 2, 10, 10, STAR_C),
        }
    }
    let p = &g.p;
    let blink = p.hurt > 0 && (g.frame / 4) % 2 == 0;
    if !blink {
        let (x, y) = (p.b.x / SUB - cam, p.b.y / SUB);
        let body = if p.star > 0 {
            [CAP, STAR_C, PLAYER][(g.frame / 3 % 3) as usize]
        } else {
            PLAYER
        };
        rect(fb, x, y, p.b.w, p.b.h, body);
        rect(fb, x, y, p.b.w, 4, CAP);
        let ex = if p.facing > 0 { x + 7 } else { x + 2 };
        rect(fb, ex, y + 6, 3, 3, WHITE);
    }
}

fn draw_hud(g: &Game, fb: &mut [u32]) {
    num(fb, 4, 4, g.score, WHITE);
    rect(fb, 110, 4, 8, 10, COIN);
    num(fb, 122, 4, g.coins, WHITE);
    rect(fb, 180, 4, 10, 10, PLAYER);
    num(fb, 194, 4, g.lives as u32, WHITE);
}

/// Numero con font 3x5 scalato x2.
fn num(fb: &mut [u32], x: i32, y: i32, mut n: u32, c: u32) {
    let mut digits = [0u8; 10];
    let mut k = 0;
    loop {
        digits[k] = (n % 10) as u8;
        k += 1;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    let mut cx = x;
    for i in (0..k).rev() {
        for (r, &bits) in FONT[digits[i] as usize].iter().enumerate() {
            for col in 0..3 {
                if bits & (4u8 >> col) != 0 {
                    rect(fb, cx + col * 2, y + r as i32 * 2, 2, 2, c);
                }
            }
        }
        cx += 8;
    }
}

fn draw_map(g: &Game, fb: &mut [u32]) {
    fill(fb, MAP_BG);
    for i in 0..LEVELS.len() {
        let x = 40 + i as i32 * 80;
        let y = 100;
        if i + 1 < LEVELS.len() {
            rect(fb, x + 24, y + 10, 56, 4, 0xFFE0C080);
        }
        let c = if i < g.unlocked { FLAG } else { 0xFF555555 };
        rect(fb, x, y, 24, 24, c);
        if i == g.cursor {
            rect(fb, x - 3, y - 3, 30, 3, WHITE);
            rect(fb, x - 3, y + 24, 30, 3, WHITE);
            rect(fb, x - 3, y - 3, 3, 30, WHITE);
            rect(fb, x + 24, y - 3, 3, 30, WHITE);
        }
        num(fb, x + 8, y + 34, i as u32 + 1, WHITE);
    }
}
