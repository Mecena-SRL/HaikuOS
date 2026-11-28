//! Interfaccia dell'editor disegnata su framebuffer `u32` (ARGB). `no_std`.
//! Il rendering è separato dalla piattaforma: il chiamante fornisce il buffer.

use crate::{History, Scene};

/// Formato pixel nativo della piattaforma (u32 ARGB sull'host, u16 BGR555 sul GBA).
pub trait Pixel: Copy {
    fn from_argb(c: u32) -> Self;
}

impl Pixel for u32 {
    fn from_argb(c: u32) -> Self {
        c
    }
}

impl Pixel for u16 {
    fn from_argb(c: u32) -> Self {
        let r = (c >> 19) & 0x1F;
        let g = (c >> 11) & 0x1F;
        let b = (c >> 3) & 0x1F;
        (r | (g << 5) | (b << 10)) as u16
    }
}

/// Area di disegno su un buffer di pixel. I colori in ingresso sono sempre ARGB.
pub struct Canvas<'a, P: Pixel> {
    pub px: &'a mut [P],
    pub w: usize,
    pub h: usize,
}

impl<P: Pixel> Canvas<'_, P> {
    pub fn clear(&mut self, c: u32) {
        let p = P::from_argb(c);
        self.px.iter_mut().for_each(|x| *x = p);
    }

    pub fn rect(&mut self, x: usize, y: usize, w: usize, h: usize, c: u32) {
        let p = P::from_argb(c);
        for yy in y..(y + h).min(self.h) {
            let row = yy * self.w;
            for xx in x..(x + w).min(self.w) {
                self.px[row + xx] = p;
            }
        }
    }

    /// Contorno di 1 pixel (usato per il cursore).
    pub fn frame(&mut self, x: usize, y: usize, w: usize, h: usize, c: u32) {
        self.rect(x, y, w, 1, c);
        self.rect(x, y + h.saturating_sub(1), w, 1, c);
        self.rect(x, y, 1, h, c);
        self.rect(x + w.saturating_sub(1), y, 1, h, c);
    }
}

/// Colori di tile (indice = valore tile). Il valore 0 è lo sfondo.
pub const PALETTE: [u32; 8] = [
    0xFF1A1A2E, 0xFF3A7D44, 0xFF8D6E63, 0xFF2B6CB0, 0xFFE0B040, 0xFFB0B0B0, 0xFF805AD5, 0xFFE53E3E,
];
/// Colore degli sprite per tipo (`kind`), ciclico.
pub const SPRITE_COLORS: [u32; 4] = [0xFFFFFFFF, 0xFFFF66CC, 0xFF66FFFF, 0xFFFF9900];
pub const CURSOR_COLOR: u32 = 0xFFFFFF00;
pub const SOLID_COLOR: u32 = 0xFFFF2020;

/// Stato dell'editor: scena, undo, cursore, pennello.
pub struct Editor<const W: usize, const H: usize, const N: usize, const U: usize> {
    pub scene: Scene<W, H, N>,
    pub hist: History<U>,
    pub cx: usize,
    pub cy: usize,
    pub brush: u8,
}

/// Tasti logici; la piattaforma li mappa dai tasti reali.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Paint,
    Undo,
    NextBrush,
    PrevBrush,
    /// Alterna il flag solido della cella.
    Solid,
    Quit,
}

impl<const W: usize, const H: usize, const N: usize, const U: usize> Editor<W, H, N, U> {
    pub fn new(scene: Scene<W, H, N>) -> Self {
        Self { scene, hist: History::default(), cx: 0, cy: 0, brush: 1 }
    }

    pub fn handle(&mut self, k: Key) {
        match k {
            Key::Up => self.cy = self.cy.saturating_sub(1),
            Key::Down => self.cy = (self.cy + 1).min(H - 1),
            Key::Left => self.cx = self.cx.saturating_sub(1),
            Key::Right => self.cx = (self.cx + 1).min(W - 1),
            Key::Paint => {
                self.scene.paint(&mut self.hist, self.cx, self.cy, self.brush);
            }
            Key::Undo => {
                self.scene.undo(&mut self.hist);
            }
            Key::Solid => {
                let (x, y) = (self.cx, self.cy);
                let v = self.scene.solid.get(x, y).unwrap_or(0);
                self.scene.solid.set(x, y, 1 - v.min(1));
            }
            Key::Quit => {}
            Key::NextBrush => self.brush = (self.brush + 1) % PALETTE.len() as u8,
            Key::PrevBrush => self.brush = (self.brush + PALETTE.len() as u8 - 1) % PALETTE.len() as u8,
        }
    }

    /// Disegna la scena con tile quadrate di `ts` pixel, in alto a sinistra.
    pub fn draw<P: Pixel>(&self, c: &mut Canvas<P>, ts: usize) {
        c.clear(0xFF000000);
        for y in 0..H {
            for x in 0..W {
                let v = self.scene.tiles.cells[y][x] as usize;
                let col = PALETTE[v % PALETTE.len()];
                c.rect(x * ts, y * ts, ts - 1, ts - 1, col);
            }
        }
        for y in 0..H {
            for x in 0..W {
                if self.scene.solid.cells[y][x] != 0 {
                    c.rect(x * ts + ts / 3, y * ts + ts / 3, ts / 3, ts / 3, SOLID_COLOR);
                }
            }
        }
        for s in self.scene.sprites.iter().flatten() {
            let col = SPRITE_COLORS[s.kind as usize % SPRITE_COLORS.len()];
            let (sx, sy) = (s.x as usize * ts + ts / 4, s.y as usize * ts + ts / 4);
            c.rect(sx, sy, ts / 2, ts / 2, col);
        }
        c.frame(self.cx * ts, self.cy * ts, ts, ts, CURSOR_COLOR);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sprite;

    type E = Editor<4, 3, 2, 8>;

    #[test]
    fn cursor_clamps_and_paints() {
        let mut e = E::new(Scene::default());
        e.handle(Key::Up);
        assert_eq!((e.cx, e.cy), (0, 0));
        e.handle(Key::Right);
        e.handle(Key::Right);
        e.handle(Key::Right);
        e.handle(Key::Right);
        assert_eq!(e.cx, 3);
        e.handle(Key::Paint);
        assert_eq!(e.scene.tiles.get(3, 0), Some(1));
        e.handle(Key::Undo);
        assert_eq!(e.scene.tiles.get(3, 0), Some(0));
    }

    #[test]
    fn solid_toggles() {
        let mut e = E::new(Scene::default());
        e.handle(Key::Solid);
        assert_eq!(e.scene.solid.get(0, 0), Some(1));
        e.handle(Key::Solid);
        assert_eq!(e.scene.solid.get(0, 0), Some(0));
    }

    #[test]
    fn draw_fills_canvas_and_marks_sprite() {
        let mut e = E::new(Scene::default());
        e.scene.add_sprite(Sprite { kind: 1, x: 1, y: 1 });
        let mut buf = [0u32; 16 * 12];
        let mut c = Canvas { px: &mut buf, w: 16, h: 12 };
        e.draw(&mut c, 4);
        // Centro dello sprite (1,1) con ts=4 -> pixel (5,5) area sprite.
        assert_eq!(buf[6 * 16 + 6], SPRITE_COLORS[1]);
        assert_eq!(buf[0], CURSOR_COLOR);
    }
}

#[cfg(test)]
mod pixel_tests {
    use super::*;

    #[test]
    fn bgr555_conversion() {
        assert_eq!(<u16 as Pixel>::from_argb(0xFFFF0000), 0x001F); // rosso
        assert_eq!(<u16 as Pixel>::from_argb(0xFF00FF00), 0x03E0); // verde
        assert_eq!(<u16 as Pixel>::from_argb(0xFF0000FF), 0x7C00); // blu
        assert_eq!(<u32 as Pixel>::from_argb(0xFF112233), 0xFF112233);
    }
}
