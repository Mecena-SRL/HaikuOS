//! Compositor di prova: copia superfici rettangolari in un framebuffer 240x160.
//! Nessuna dipendenza da hardware: il test host controlla i byte prodotti.
//! Ordine: la superficie successiva nello slice sta sopra quella precedente.

pub const W: usize = 240;
pub const H: usize = 160;

#[derive(Clone, Copy)]
pub struct Surface<'a> {
    pub x: i32,
    pub y: i32,
    pub w: usize,
    pub h: usize,
    /// Pixel in ordine di riga, lunghi `w * h`.
    pub pixels: &'a [u16],
}

/// Disegna le superfici in `fb` (`W * H` pixel). Ritorna quanti pixel sono stati scritti.
/// Le parti fuori schermo vengono tagliate. Superfici con pixel mancanti sono ignorate.
pub fn compose(fb: &mut [u16], surfaces: &[Surface]) -> usize {
    assert!(fb.len() >= W * H);
    let mut written = 0;
    for s in surfaces {
        if s.pixels.len() < s.w * s.h {
            continue;
        }
        // Intervallo visibile sullo schermo.
        let x0 = s.x.max(0) as usize;
        let y0 = s.y.max(0) as usize;
        let x1 = (s.x + s.w as i32).clamp(0, W as i32) as usize;
        let y1 = (s.y + s.h as i32).clamp(0, H as i32) as usize;
        if x0 >= x1 || y0 >= y1 {
            continue;
        }
        for y in y0..y1 {
            let src_row = (y as i32 - s.y) as usize * s.w;
            let dst_row = y * W;
            for x in x0..x1 {
                fb[dst_row + x] = s.pixels[src_row + (x as i32 - s.x) as usize];
                written += 1;
            }
        }
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fb() -> std::vec::Vec<u16> {
        std::vec![0u16; W * H]
    }

    #[test]
    fn una_superficie_nel_punto_giusto() {
        let px = [1u16, 2, 3, 4];
        let mut f = fb();
        let n = compose(&mut f, &[Surface { x: 10, y: 5, w: 2, h: 2, pixels: &px }]);
        assert_eq!(n, 4);
        assert_eq!(f[5 * W + 10], 1);
        assert_eq!(f[5 * W + 11], 2);
        assert_eq!(f[6 * W + 10], 3);
        assert_eq!(f[6 * W + 11], 4);
        assert_eq!(f[5 * W + 9], 0);
        assert_eq!(f[4 * W + 10], 0);
    }

    #[test]
    fn la_superficie_successiva_sta_sopra() {
        let a = [7u16; 4];
        let b = [9u16; 4];
        let mut f = fb();
        compose(
            &mut f,
            &[
                Surface { x: 0, y: 0, w: 2, h: 2, pixels: &a },
                Surface { x: 1, y: 1, w: 2, h: 2, pixels: &b },
            ],
        );
        assert_eq!(f[0], 7);
        assert_eq!(f[W + 1], 9);
        assert_eq!(f[2 * W + 2], 9);
        assert_eq!(f[1], 7);
    }

    #[test]
    fn taglio_ai_bordi() {
        let px = [5u16; 16];
        let mut f = fb();
        let n = compose(&mut f, &[Surface { x: -2, y: 158, w: 4, h: 4, pixels: &px }]);
        assert_eq!(n, 2 * 2);
        assert_eq!(f[158 * W], 5);
        assert_eq!(f[159 * W + 1], 5);
        assert_eq!(f[159 * W + 2], 0);
    }

    #[test]
    fn fuori_schermo_non_scrive() {
        let px = [5u16; 4];
        let mut f = fb();
        let n = compose(
            &mut f,
            &[
                Surface { x: 240, y: 0, w: 2, h: 2, pixels: &px },
                Surface { x: 0, y: -2, w: 2, h: 2, pixels: &px },
            ],
        );
        assert_eq!(n, 0);
        assert!(f.iter().all(|&p| p == 0));
    }

    #[test]
    fn pixel_mancanti_ignorati() {
        let px = [5u16; 2];
        let mut f = fb();
        assert_eq!(compose(&mut f, &[Surface { x: 0, y: 0, w: 2, h: 2, pixels: &px }]), 0);
    }
}
