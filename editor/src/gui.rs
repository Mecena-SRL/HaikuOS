//! Interfaccia dell'editor disegnata su framebuffer `u32` (ARGB). `no_std`.
//! Il rendering è separato dalla piattaforma: il chiamante fornisce il buffer.

use crate::{History, Scene};

/// Area di disegno su un buffer di pixel.
pub struct Canvas<'a> {
    pub px: &'a mut [u32],
    pub w: usize,
    pub h: usize,
}

impl Canvas<'_> {
    pub fn clear(&mut self, c: u32) {
        self.px.iter_mut().for_each(|p| *p = c);
    }

    pub fn rect(&mut self, x: usize, y: usize, w: usize, h: usize, c: u32) {
        for yy in y..(y + h).min(self.h) {
            let row = yy * self.w;
            for xx in x..(x + w).min(self.w) {
                self.px[row + xx] = c;
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
            Key::Quit => {}
            Key::NextBrush => self.brush = (self.brush + 1) % PALETTE.len() as u8,
            Key::PrevBrush => self.brush = (self.brush + PALETTE.len() as u8 - 1) % PALETTE.len() as u8,
        }
    }

    /// Disegna la scena con tile quadrate di `ts` pixel, in alto a sinistra.
    pub fn draw(&self, c: &mut Canvas, ts: usize) {
        c.clear(0xFF000000);
        for y in 0..H {
            for x in 0..W {
                let v = self.scene.tiles.cells[y][x] as usize;
                let col = PALETTE[v % PALETTE.len()];
                c.rect(x * ts, y * ts, ts - 1, ts - 1, col);
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
