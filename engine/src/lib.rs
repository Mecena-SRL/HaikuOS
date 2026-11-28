//! Motore di gioco HaikuOS, fase 1: core fisico deterministico e renderer software.
//! `no_std`, senza allocazione, senza dipendenze. Nessuna GPU: vedi docs/engine.md.
#![cfg_attr(not(test), no_std)]

pub mod replay;

/// Numero in virgola fissa Q16.16. Stessi bit su ogni piattaforma: la simulazione è deterministica.
pub type Fx = i32;
pub const ONE: Fx = 1 << 16;

#[inline]
pub const fn px(p: i32) -> Fx {
    p << 16
}
/// Pixel interi (arrotondamento verso il basso).
#[inline]
pub const fn to_px(f: Fx) -> i32 {
    f >> 16
}

/// Gravità per tick (1/16 di pixel al quadrato) e velocità massima di caduta.
pub const GRAVITY: Fx = ONE / 16;
pub const MAX_FALL: Fx = 4 * ONE;

/// Mappa a tile. `cells[i] == 0` è vuoto; ogni altro valore è un tile solido con quell'indice di colore.
/// Fuori dalla mappa è tutto solido (muri invisibili).
pub struct Map<'a> {
    pub w: usize,
    pub h: usize,
    pub tile: i32,
    pub cells: &'a [u8],
}

impl Map<'_> {
    #[inline]
    pub fn cell(&self, tx: i32, ty: i32) -> u8 {
        if tx < 0 || ty < 0 || tx as usize >= self.w || ty as usize >= self.h {
            return 1;
        }
        self.cells[ty as usize * self.w + tx as usize]
    }

    /// True se il rettangolo in pixel (x, y, w, h) tocca un tile solido.
    pub fn hits(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        let t = self.tile;
        for ty in y.div_euclid(t)..=(y + h - 1).div_euclid(t) {
            for tx in x.div_euclid(t)..=(x + w - 1).div_euclid(t) {
                if self.cell(tx, ty) != 0 {
                    return true;
                }
            }
        }
        false
    }
}

/// Corpo con AABB in pixel. Posizione e velocità in Q16.16.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Body {
    pub x: Fx,
    pub y: Fx,
    pub vx: Fx,
    pub vy: Fx,
    pub w: i32,
    pub h: i32,
    pub on_ground: bool,
}

/// Avanza `b` di `d` (Q16.16) su un asse, a passi di al più 1 pixel.
/// Al primo contatto azzera la velocità su quell'asse e ritorna true.
fn sweep(b: &mut Body, m: &Map, d: Fx, horizontal: bool) -> bool {
    let mut rem = d;
    while rem != 0 {
        let step = rem.clamp(-ONE, ONE);
        rem -= step;
        let (nx, ny) = if horizontal { (b.x + step, b.y) } else { (b.x, b.y + step) };
        if m.hits(to_px(nx), to_px(ny), b.w, b.h) {
            if horizontal {
                b.vx = 0;
            } else {
                b.vy = 0;
            }
            return true;
        }
        b.x = nx;
        b.y = ny;
    }
    false
}

/// Un tick fisso di simulazione: gravità, poi X, poi Y. Non dipende dal tempo reale.
pub fn step(b: &mut Body, m: &Map) {
    // Un corpo nato dentro un solido viene spinto su di 1 px alla volta (al massimo 16 px).
    for _ in 0..16 {
        if !m.hits(to_px(b.x), to_px(b.y), b.w, b.h) {
            break;
        }
        b.y -= ONE;
    }
    b.vy = (b.vy + GRAVITY).min(MAX_FALL);
    sweep(b, m, b.vx, true);
    let falling = b.vy > 0;
    // appoggiato solo se la caduta è stata bloccata
    b.on_ground = sweep(b, m, b.vy, false) && falling;
}

/// Frame di disegno su buffer ARGB32. Ritaglia tutto nei bordi.
pub struct Frame<'a> {
    pub w: usize,
    pub h: usize,
    pub px: &'a mut [u32],
}

impl Frame<'_> {
    pub fn clear(&mut self, c: u32) {
        self.px.fill(c);
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        let (sw, sh) = (self.w as i32, self.h as i32);
        let x0 = x.clamp(0, sw) as usize;
        let y0 = y.clamp(0, sh) as usize;
        let x1 = (x + w).clamp(0, sw) as usize;
        let y1 = (y + h).clamp(0, sh) as usize;
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        for row in y0..y1 {
            self.px[row * self.w + x0..row * self.w + x1].fill(c);
        }
    }

    /// Disegna la mappa. `cam_x`/`cam_y` sono la posizione della camera in pixel;
    /// `pal[i]` è il colore del tile di indice `i`.
    pub fn draw_map(&mut self, m: &Map, pal: &[u32], cam_x: i32, cam_y: i32) {
        let t = m.tile;
        for ty in 0..m.h as i32 {
            for tx in 0..m.w as i32 {
                let c = m.cells[ty as usize * m.w + tx as usize];
                if c != 0 {
                    let c = pal.get(c as usize).copied().unwrap_or(0xFFFF_00FF);
                    self.fill_rect(tx * t - cam_x, ty * t - cam_y, t, t, c);
                }
            }
        }
    }

    /// Sprite ARGB con trasparenza: il pixel 0 non viene disegnato.
    pub fn blit(&mut self, src: &Sprite, x: i32, y: i32) {
        for sy in 0..src.h {
            let dy = y + sy as i32;
            if dy < 0 || dy >= self.h as i32 {
                continue;
            }
            for sx in 0..src.w {
                let dx = x + sx as i32;
                let c = src.px[sy * src.w + sx];
                if c != 0 && dx >= 0 && dx < self.w as i32 {
                    self.px[dy as usize * self.w + dx as usize] = c;
                }
            }
        }
    }
}

pub struct Sprite<'a> {
    pub w: usize,
    pub h: usize,
    pub px: &'a [u32],
}

/// Hash FNV-1a dello stato di un corpo: serve per verificare il determinismo tra piattaforme.
pub fn state_hash(bodies: &[Body]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in bodies {
        for v in [b.x, b.y, b.vx, b.vy] {
            for byte in v.to_le_bytes() {
                h = (h ^ byte as u32).wrapping_mul(0x0100_0193);
            }
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: usize = 8;
    const H: usize = 6;
    // pavimento a riga 5, muro a colonna 7
    fn level() -> [u8; W * H] {
        let mut c = [0u8; W * H];
        for x in 0..W {
            c[5 * W + x] = 1;
        }
        for y in 0..H {
            c[y * W + 7] = 1;
        }
        c
    }

    #[test]
    fn falls_and_lands_on_floor() {
        let cells = level();
        let m = Map { w: W, h: H, tile: 16, cells: &cells };
        let mut b = Body { x: px(16), y: px(0), w: 8, h: 8, ..Default::default() };
        for _ in 0..200 {
            step(&mut b, &m);
        }
        assert!(b.on_ground);
        assert_eq!(to_px(b.y) + b.h, 5 * 16); // appoggiato sul bordo superiore del pavimento
    }

    #[test]
    fn body_spawned_inside_solid_is_freed() {
        let cells = level();
        let m = Map { w: W, h: H, tile: 16, cells: &cells };
        // nato dentro il pavimento (riga 5 = y 80..96)
        let mut b = Body { x: px(16), y: px(84), w: 8, h: 8, ..Default::default() };
        step(&mut b, &m);
        assert!(!m.hits(to_px(b.x), to_px(b.y), b.w, b.h));
        for _ in 0..200 {
            step(&mut b, &m);
        }
        assert!(b.on_ground);
    }

    #[test]
    fn wall_stops_horizontal_motion() {
        let cells = level();
        let m = Map { w: W, h: H, tile: 16, cells: &cells };
        let mut b = Body { x: px(16), y: px(16), vx: ONE, w: 8, h: 8, ..Default::default() };
        for _ in 0..300 {
            step(&mut b, &m);
        }
        assert!(to_px(b.x) + b.w <= 7 * 16);
        assert_eq!(b.vx, 0);
    }

    #[test]
    fn simulation_is_deterministic() {
        let cells = level();
        let m = Map { w: W, h: H, tile: 16, cells: &cells };
        let run = || {
            let mut bs = [Body { x: px(20), y: px(2), vx: 3 * ONE / 4, w: 6, h: 6, ..Default::default() }; 3];
            for (i, b) in bs.iter_mut().enumerate() {
                b.x += px(i as i32 * 3);
            }
            for _ in 0..500 {
                for b in bs.iter_mut() {
                    step(b, &m);
                }
            }
            state_hash(&bs)
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn fill_rect_outside_right_and_bottom_does_not_panic() {
        let mut buf = [0u32; 4 * 4];
        let mut f = Frame { w: 4, h: 4, px: &mut buf };
        f.fill_rect(10, 1, 2, 2, 0xFF00_0001); // tutto a destra
        f.fill_rect(1, 9, 2, 2, 0xFF00_0002); // tutto sotto
        f.fill_rect(3, 3, 100, 100, 0xFF00_0003); // taglio in basso a destra
        assert_eq!(buf.iter().filter(|&&c| c == 0xFF00_0003).count(), 1);
        assert_eq!(buf.iter().filter(|&&c| c == 0xFF00_0001 || c == 0xFF00_0002).count(), 0);
    }

    #[test]
    fn frame_clips_and_skips_transparent() {
        let mut buf = [0u32; 4 * 4];
        let mut f = Frame { w: 4, h: 4, px: &mut buf };
        f.fill_rect(-2, -2, 4, 4, 0xFF11_2233); // angolo in alto a sinistra visibile: 2x2
        assert_eq!(buf.iter().filter(|&&c| c == 0xFF11_2233).count(), 4);
        let sp = [0u32, 0xFFAA_BBCC, 0xFFAA_BBCC, 0];
        let mut buf2 = [0u32; 4 * 4];
        let mut f2 = Frame { w: 4, h: 4, px: &mut buf2 };
        f2.blit(&Sprite { w: 2, h: 2, px: &sp }, 2, 2); // 2 pixel opachi visibili, 2 trasparenti
        assert_eq!(buf2.iter().filter(|&&c| c != 0).count(), 2);
    }
}
