//! Motore dati dell'editor di giochi di HaikuOS.
//! `no_std`, nessuna allocazione: tutto ha dimensione fissa, adatto a GBA.

#![cfg_attr(not(test), no_std)]

pub mod gui;
pub mod platform;

/// Griglia di tile, `cells[y][x]`. Valore 0 = vuoto.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Grid<const W: usize, const H: usize> {
    pub cells: [[u8; W]; H],
}

impl<const W: usize, const H: usize> Default for Grid<W, H> {
    fn default() -> Self {
        Self { cells: [[0; W]; H] }
    }
}

impl<const W: usize, const H: usize> Grid<W, H> {
    pub fn get(&self, x: usize, y: usize) -> Option<u8> {
        self.cells.get(y)?.get(x).copied()
    }

    /// Imposta una cella e restituisce il valore precedente.
    pub fn set(&mut self, x: usize, y: usize, v: u8) -> Option<u8> {
        let c = self.cells.get_mut(y)?.get_mut(x)?;
        Some(core::mem::replace(c, v))
    }

    /// Riempie il rettangolo [x0..=x1] x [y0..=y1], ritagliato ai bordi.
    pub fn fill_rect(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, v: u8) {
        for row in self.cells.iter_mut().take(y1 + 1).skip(y0) {
            for c in row.iter_mut().take(x1 + 1).skip(x0) {
                *c = v;
            }
        }
    }
}

/// Oggetto posizionato (nemico, giocatore, pedina...). `kind` lo interpreta il gioco.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Sprite {
    pub kind: u8,
    pub x: u16,
    pub y: u16,
}

/// Una scena: sfondo a tile + al massimo `N` sprite.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Scene<const W: usize, const H: usize, const N: usize> {
    pub tiles: Grid<W, H>,
    pub sprites: [Option<Sprite>; N],
}

impl<const W: usize, const H: usize, const N: usize> Default for Scene<W, H, N> {
    fn default() -> Self {
        Self { tiles: Grid::default(), sprites: [None; N] }
    }
}

/// Modifica di una tile, per annullare (undo).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Edit {
    pub x: u16,
    pub y: u16,
    pub old: u8,
    pub new: u8,
}

/// Pila di undo a dimensione fissa: oltre `N` modifiche perde la più vecchia.
pub struct History<const N: usize> {
    ops: [Option<Edit>; N],
    len: usize,
}

impl<const N: usize> Default for History<N> {
    fn default() -> Self {
        Self { ops: [None; N], len: 0 }
    }
}

impl<const N: usize> History<N> {
    fn push(&mut self, e: Edit) {
        if N == 0 {
            return;
        }
        if self.len == N {
            self.ops.copy_within(1.., 0);
            self.len -= 1;
        }
        self.ops[self.len] = Some(e);
        self.len += 1;
    }

    fn pop(&mut self) -> Option<Edit> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        self.ops[self.len].take()
    }
}

impl<const W: usize, const H: usize, const N: usize> Scene<W, H, N> {
    /// Dipinge una tile. Registra l'undo solo se il valore cambia.
    pub fn paint<const M: usize>(
        &mut self,
        hist: &mut History<M>,
        x: usize,
        y: usize,
        v: u8,
    ) -> bool {
        match self.tiles.get(x, y) {
            Some(old) if old != v => {
                self.tiles.set(x, y, v);
                hist.push(Edit { x: x as u16, y: y as u16, old, new: v });
                true
            }
            _ => false,
        }
    }

    /// Annulla l'ultima modifica di tile.
    pub fn undo<const M: usize>(&mut self, hist: &mut History<M>) -> bool {
        let Some(e) = hist.pop() else { return false };
        self.tiles.set(e.x as usize, e.y as usize, e.old);
        true
    }

    /// Inserisce uno sprite nel primo slot libero.
    pub fn add_sprite(&mut self, s: Sprite) -> Option<usize> {
        let i = self.sprites.iter().position(|slot| slot.is_none())?;
        self.sprites[i] = Some(s);
        Some(i)
    }

    pub fn remove_sprite(&mut self, i: usize) -> Option<Sprite> {
        self.sprites.get_mut(i)?.take()
    }

    /// Dimensione del formato di salvataggio.
    pub const fn encoded_len() -> usize {
        4 + W * H + N * 6
    }

    /// Salva in `out`: magic, tile per righe, sprite a 6 byte ciascuno.
    pub fn encode(&self, out: &mut [u8]) -> Option<usize> {
        let out = out.get_mut(..Self::encoded_len())?;
        out[..4].copy_from_slice(&MAGIC);
        let mut p = 4;
        for row in self.tiles.cells.iter() {
            out[p..p + W].copy_from_slice(row);
            p += W;
        }
        for slot in self.sprites.iter() {
            let rec = match slot {
                Some(s) => {
                    let [xl, xh] = s.x.to_le_bytes();
                    let [yl, yh] = s.y.to_le_bytes();
                    [1, s.kind, xl, xh, yl, yh]
                }
                None => [0; 6],
            };
            out[p..p + 6].copy_from_slice(&rec);
            p += 6;
        }
        Some(Self::encoded_len())
    }

    /// Carica da un buffer salvato con `encode`. `None` se non valido.
    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() != Self::encoded_len() || buf[..4] != MAGIC {
            return None;
        }
        let mut s = Self::default();
        let mut p = 4;
        for row in s.tiles.cells.iter_mut() {
            row.copy_from_slice(&buf[p..p + W]);
            p += W;
        }
        for slot in s.sprites.iter_mut() {
            let r = &buf[p..p + 6];
            p += 6;
            if r[0] == 1 {
                *slot = Some(Sprite {
                    kind: r[1],
                    x: u16::from_le_bytes([r[2], r[3]]),
                    y: u16::from_le_bytes([r[4], r[5]]),
                });
            }
        }
        Some(s)
    }
}

pub const MAGIC: [u8; 4] = *b"HGS1";

#[cfg(test)]
mod tests {
    use super::*;

    type S = Scene<8, 6, 4>;

    #[test]
    fn paint_and_undo() {
        let mut s = S::default();
        let mut h = History::<8>::default();
        assert!(s.paint(&mut h, 2, 3, 7));
        assert!(!s.paint(&mut h, 2, 3, 7)); // stesso valore: nessun undo
        assert_eq!(s.tiles.get(2, 3), Some(7));
        assert!(s.undo(&mut h));
        assert_eq!(s.tiles.get(2, 3), Some(0));
        assert!(!s.undo(&mut h));
    }

    #[test]
    fn history_drops_oldest() {
        let mut s = S::default();
        let mut h = History::<2>::default();
        s.paint(&mut h, 0, 0, 1);
        s.paint(&mut h, 1, 0, 1);
        s.paint(&mut h, 2, 0, 1);
        assert!(s.undo(&mut h));
        assert!(s.undo(&mut h));
        assert!(!s.undo(&mut h)); // il primo è stato scartato
        assert_eq!(s.tiles.get(0, 0), Some(1));
    }

    #[test]
    fn save_load_roundtrip() {
        let mut s = S::default();
        let mut h = History::<1>::default();
        s.paint(&mut h, 5, 4, 9);
        s.add_sprite(Sprite { kind: 3, x: 300, y: 2 });
        let mut buf = [0u8; 256];
        let n = s.encode(&mut buf).unwrap();
        assert_eq!(n, S::encoded_len());
        assert_eq!(S::decode(&buf[..n]), Some(s));
        assert_eq!(S::decode(&buf[..n - 1]), None);
    }

    #[test]
    fn fill_rect_clips_bounds() {
        let mut g = Grid::<4, 4>::default();
        g.fill_rect(2, 2, 9, 9, 5);
        assert_eq!(g.get(3, 3), Some(5));
        assert_eq!(g.get(1, 1), Some(0));
    }
}
