//! API di piattaforma minima. Ogni backend (host, framebuffer HaikuOS, GBA)
//! implementa questo trait; il ciclo dell'editor è uguale per tutti.

use crate::gui::{Canvas, Editor, Key, Pixel};

pub trait Platform {
    /// Formato del framebuffer: u32 sull'host, u16 sul GBA.
    type Px: Pixel;
    /// Dimensioni del framebuffer in pixel.
    fn size(&self) -> (usize, usize);
    /// Buffer dove disegnare il frame corrente.
    fn buffer(&mut self) -> &mut [Self::Px];
    /// Prossimo tasto, `None` quando l'input è finito.
    fn poll_key(&mut self) -> Option<Key>;
    /// Mostra il frame disegnato.
    fn present(&mut self);
}

/// Ciclo principale: input -> stato -> disegno -> mostra. Ritorna al tasto `Quit`.
pub fn run<const W: usize, const H: usize, const N: usize, const U: usize, P: Platform>(
    ed: &mut Editor<W, H, N, U>,
    p: &mut P,
    ts: usize,
) {
    while let Some(k) = p.poll_key() {
        if k == Key::Quit {
            break;
        }
        ed.handle(k);
        let (w, h) = p.size();
        ed.draw(&mut Canvas { px: p.buffer(), w, h }, ts);
        p.present();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        keys: [Option<Key>; 4],
        n: usize,
        buf: [u32; 8 * 6],
        presents: usize,
    }

    impl Platform for Fake {
        type Px = u32;
        fn size(&self) -> (usize, usize) {
            (8, 6)
        }
        fn buffer(&mut self) -> &mut [u32] {
            &mut self.buf
        }
        fn poll_key(&mut self) -> Option<Key> {
            let k = self.keys.get(self.n).copied().flatten();
            self.n += 1;
            k
        }
        fn present(&mut self) {
            self.presents += 1;
        }
    }

    #[test]
    fn run_handles_keys_until_quit() {
        let mut ed: Editor<8, 6, 2, 4> = Editor::new(Default::default());
        let mut p = Fake { keys: [Some(Key::Right), Some(Key::Paint), Some(Key::Quit), Some(Key::Up)], n: 0, buf: [0; 48], presents: 0 };
        run(&mut ed, &mut p, 4);
        assert_eq!((ed.cx, ed.scene.tiles.get(1, 0)), (1, Some(1)));
        assert_eq!(p.presents, 2); // dopo Quit non c'è più disegno
    }
}
