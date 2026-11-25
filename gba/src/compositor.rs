//! Compositor di prova: copia superfici rettangolari in un framebuffer 240x160.
//! Nessuna dipendenza da hardware: il test host controlla i byte prodotti.
//! Ordine: la superficie successiva nello slice sta sopra quella precedente.
//! Trasparenza: un pixel uguale alla chiave `key` non viene copiato.

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
    /// Colore trasparente: i pixel uguali a questo valore lasciano vedere lo sfondo.
    pub key: Option<u16>,
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
                let c = s.pixels[src_row + (x as i32 - s.x) as usize];
                if s.key == Some(c) {
                    continue;
                }
                fb[dst_row + x] = c;
                written += 1;
            }
        }
    }
    written
}

/// Riempie un rettangolo con un colore solido, tagliato ai bordi. Ritorna i pixel scritti.
pub fn fill_rect(fb: &mut [u16], x: i32, y: i32, w: usize, h: usize, c: u16) -> usize {
    assert!(fb.len() >= W * H);
    let x0 = x.max(0) as usize;
    let y0 = y.max(0) as usize;
    let x1 = (x + w as i32).clamp(0, W as i32) as usize;
    let y1 = (y + h as i32).clamp(0, H as i32) as usize;
    if x0 >= x1 || y0 >= y1 {
        return 0;
    }
    for row in fb[y0 * W..y1 * W].chunks_mut(W) {
        row[x0..x1].fill(c);
    }
    (x1 - x0) * (y1 - y0)
}

// Colori di prova (BGR555 del GBA). Da verificare sullo schermo reale.
pub const C_FONDO: u16 = 0x2D6B;
pub const C_BARRA: u16 = 0x0C63;
pub const C_TITOLO: u16 = 0x3A10;
pub const C_CORPO: u16 = 0x7FFF;
pub const C_ICONA: u16 = 0x03E0;

/// Schermata di prova del desktop: sfondo, barra in alto, una finestra, un'icona.
/// Misure (da verificare su GBA): barra 240x12, finestra 120x80 in (40,30), icona 16x16 in (8,20).
pub fn desktop_sketch(fb: &mut [u16]) {
    fill_rect(fb, 0, 0, W, H, C_FONDO);
    fill_rect(fb, 0, 0, W, 12, C_BARRA);
    fill_rect(fb, 40, 30, 120, 80, C_CORPO);
    fill_rect(fb, 40, 30, 120, 12, C_TITOLO);
    fill_rect(fb, 8, 20, 16, 16, C_ICONA);
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
        let n = compose(&mut f, &[Surface { x: 10, y: 5, w: 2, h: 2, pixels: &px, key: None }]);
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
                Surface { x: 0, y: 0, w: 2, h: 2, pixels: &a, key: None },
                Surface { x: 1, y: 1, w: 2, h: 2, pixels: &b, key: None },
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
        let n = compose(&mut f, &[Surface { x: -2, y: 158, w: 4, h: 4, pixels: &px, key: None }]);
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
                Surface { x: 240, y: 0, w: 2, h: 2, pixels: &px, key: None },
                Surface { x: 0, y: -2, w: 2, h: 2, pixels: &px, key: None },
            ],
        );
        assert_eq!(n, 0);
        assert!(f.iter().all(|&p| p == 0));
    }

    #[test]
    fn pixel_trasparenti_lasciano_vedere_lo_sfondo() {
        let sotto = [3u16; 4];
        let sopra = [8u16, 0xFFFF, 0xFFFF, 8];
        let mut f = fb();
        let n = compose(
            &mut f,
            &[
                Surface { x: 0, y: 0, w: 2, h: 2, pixels: &sotto, key: None },
                Surface { x: 0, y: 0, w: 2, h: 2, pixels: &sopra, key: Some(0xFFFF) },
            ],
        );
        assert_eq!(n, 4 + 2);
        assert_eq!(f[0], 8);
        assert_eq!(f[1], 3);
        assert_eq!(f[W], 3);
        assert_eq!(f[W + 1], 8);
    }

    #[test]
    fn pixel_mancanti_ignorati() {
        let px = [5u16; 2];
        let mut f = fb();
        assert_eq!(compose(&mut f, &[Surface { x: 0, y: 0, w: 2, h: 2, pixels: &px, key: None }]), 0);
    }

    /// Schizzo del desktop (docs/desktop-concept.md): sfondo, barra, finestra, icona trasparente.
    #[test]
    fn scena_desktop_di_prova() {
        const BG: u16 = 2 | (4 << 5) | (10 << 10);
        const BAR: u16 = 20 | (22 << 5) | (26 << 10);
        const TITLE: u16 = 4 | (12 << 5) | (28 << 10);
        const BODY: u16 = 28 | (28 << 5) | (24 << 10);
        const ICON: u16 = 31 | (20 << 5);
        const KEY: u16 = 0x7FFF;

        let bg = std::vec![BG; W * H];
        let bar = std::vec![BAR; W * 12];
        let win: std::vec::Vec<u16> = (0..120 * 80)
            .map(|i| if i / 120 < 10 { TITLE } else { BODY })
            .collect();
        let mut icon = std::vec![ICON; 16 * 16];
        icon[0] = KEY; // angolo trasparente

        let mut f = fb();
        compose(
            &mut f,
            &[
                Surface { x: 0, y: 0, w: W, h: H, pixels: &bg, key: None },
                Surface { x: 0, y: 0, w: W, h: 12, pixels: &bar, key: None },
                Surface { x: 40, y: 30, w: 120, h: 80, pixels: &win, key: None },
                Surface { x: 8, y: 20, w: 16, h: 16, pixels: &icon, key: Some(KEY) },
            ],
        );
        assert_eq!(f[0], BAR); // barra in alto
        assert_eq!(f[11 * W + 100], BAR);
        assert_eq!(f[12 * W + 100], BG); // sotto la barra: sfondo
        assert_eq!(f[35 * W + 50], TITLE); // titolo finestra
        assert_eq!(f[50 * W + 50], BODY); // corpo finestra
        assert_eq!(f[29 * W + 50], BG); // sopra la finestra: sfondo
        assert_eq!(f[20 * W + 8], BG); // angolo trasparente dell'icona
        assert_eq!(f[20 * W + 9], ICON); // resto dell'icona
    }

    #[test]
    fn schermata_desktop_ha_barra_finestra_icona() {
        let mut f = fb();
        desktop_sketch(&mut f);
        assert_eq!(f[0], C_BARRA);
        assert_eq!(f[11 * W + 239], C_BARRA);
        assert_eq!(f[12 * W], C_FONDO);
        assert_eq!(f[30 * W + 40], C_TITOLO);
        assert_eq!(f[50 * W + 50], C_CORPO);
        assert_eq!(f[20 * W + 8], C_ICONA);
        assert_eq!(f[150 * W + 200], C_FONDO);
    }
}
