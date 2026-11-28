//! Rendering su framebuffer ARGB (VIEW_W x VIEW_H). Legge lo stato, non lo modifica.
use crate::levels::LEVELS;
use crate::*;
use haiku_gui::layout::Rect;
use haiku_gui::palette::Tok;
use haiku_gui::render::Canvas;

const TEXT_PX: i32 = 8;

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

/// Canvas sul framebuffer ARGB (VIEW_W x VIEW_H): rettangoli, angoli arrotondati e testo 3x5 di gba.
pub struct Fb<'a>(pub &'a mut [u32]);

fn argb(t: Tok) -> u32 {
    0xFF00_0000 | t.rgb()
}

impl Canvas for Fb<'_> {
    fn fill(&mut self, r: Rect, t: Tok) {
        let c = argb(t);
        for y in r.y.max(0)..(r.y + r.h).min(VIEW_H) {
            for x in r.x.max(0)..(r.x + r.w).min(VIEW_W) {
                self.0[(y * VIEW_W + x) as usize] = c;
            }
        }
    }

    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok) {
        self.vtext(x, y, TEXT_PX, s, t);
    }

    fn blend(&mut self, x: i32, y: i32, a: u8, t: Tok) {
        if x < 0 || y < 0 || x >= VIEW_W || y >= VIEW_H {
            return;
        }
        let c = t.rgb();
        let p = &mut self.0[(y * VIEW_W + x) as usize];
        let a = a as u32;
        let mix = |d: u32, s: u32| (s * a + d * (255 - a)) / 255;
        let d = *p;
        *p = 0xFF00_0000
            | (mix((d >> 16) & 0xFF, (c >> 16) & 0xFF) << 16)
            | (mix((d >> 8) & 0xFF, (c >> 8) & 0xFF) << 8)
            | mix(d & 0xFF, c & 0xFF);
    }
}

/// Disegna lo stato corrente di `g` in `fb` (VIEW_W * VIEW_H pixel).
pub fn draw(g: &Game, fb: &mut [u32]) {
    if g.state == State::Map {
        return draw_map(g, &mut Fb(fb));
    }
    fill(fb, SKY);
    draw_level(g, fb);
    let mut c = Fb(fb);
    draw_hud(g, &mut c);
    match g.state {
        State::Clear => banner(&mut c, Tok::A(8)),
        State::Over => banner(&mut c, Tok::A(11)),
        State::Won => banner(&mut c, Tok::A(1)),
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

fn banner(c: &mut Fb, t: Tok) {
    c.rect_rounded(Rect::new(48, 100, 160, 40), 6, t);
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

/// HUD: barra superiore con punteggio, monete e vite. Testo senza allocazioni.
fn draw_hud(g: &Game, c: &mut Fb) {
    let mut b = [0u8; 10];
    c.fill(Rect::new(0, 0, VIEW_W, 14), Tok::T0);
    c.text(4, 4, num_str(g.score, &mut b), Tok::T1);
    c.rect_rounded(Rect::new(110, 3, 8, 10), 2, Tok::A(1));
    c.text(122, 4, num_str(g.coins, &mut b), Tok::T1);
    c.rect_rounded(Rect::new(180, 3, 10, 10), 2, Tok::A(4));
    c.text(194, 4, num_str(g.lives as u32, &mut b), Tok::T1);
}

/// Cifre decimali in `buf`, senza allocare: restituisce la parte usata.
fn num_str(mut n: u32, buf: &mut [u8; 10]) -> &str {
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    core::str::from_utf8(&buf[i..]).unwrap_or("")
}

fn draw_map(g: &Game, c: &mut Fb) {
    let mut b = [0u8; 10];
    c.fill(Rect::new(0, 0, VIEW_W, VIEW_H), Tok::T0);
    for i in 0..LEVELS.len() {
        let x = 40 + i as i32 * 80;
        let y = 100;
        if i + 1 < LEVELS.len() {
            c.fill(Rect::new(x + 24, y + 10, 56, 4), Tok::T3);
        }
        let t = if i < g.unlocked { Tok::A(8) } else { Tok::A(6) };
        c.rect_rounded(Rect::new(x, y, 24, 24), 4, t);
        if i == g.cursor {
            c.rect_rounded(Rect::new(x - 3, y - 3, 30, 3), 1, Tok::T1);
            c.rect_rounded(Rect::new(x - 3, y + 24, 30, 3), 1, Tok::T1);
            c.rect_rounded(Rect::new(x - 3, y - 3, 3, 30), 1, Tok::T1);
            c.rect_rounded(Rect::new(x + 24, y - 3, 3, 30), 1, Tok::T1);
        }
        c.text(x + 8, y + 34, num_str(i as u32 + 1, &mut b), Tok::T1);
    }
}
