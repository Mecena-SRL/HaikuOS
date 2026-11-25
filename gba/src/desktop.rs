//! Schermata del desktop dal concept (docs/desktop-concept.md, sezioni 2 e 3).
//! Tema scuro e freddo. Disegna in un framebuffer 240x160, senza dipendenze da hardware.
#![allow(dead_code)]

pub const W: usize = 240;
pub const H: usize = 160;

const fn rgb(r: u16, g: u16, b: u16) -> u16 {
    r | (g << 5) | (b << 10)
}

pub const BG: u16 = rgb(2, 4, 8);
pub const BAR: u16 = rgb(4, 8, 16);
pub const BAR_SLOT: u16 = rgb(8, 16, 26);
pub const CLOCK: u16 = rgb(12, 22, 31);
pub const ICON: u16 = rgb(8, 16, 26);
pub const ICON_HI: u16 = rgb(10, 22, 31);
pub const WIN_TITLE: u16 = rgb(6, 14, 24);
pub const WIN_BODY: u16 = rgb(3, 6, 12);
pub const WIN_BORDER: u16 = rgb(12, 22, 31);
pub const CLOSE: u16 = rgb(16, 18, 22);
pub const TEXT: u16 = rgb(20, 26, 31);

/// Barra superiore, in unità GBA (1 unità = 1 px).
pub const BAR_H: usize = 12;
/// Icone 16x16, griglia con passo 48 px (dal concept).
pub const ICON_SIZE: usize = 16;
pub const ICON_STEP_X: usize = 48;
pub const ICON_STEP_Y: usize = 40;
/// Finestra del terminale: posizione e misure.
pub const WIN_X: usize = 124;
pub const WIN_Y: usize = 30;
pub const WIN_W: usize = 108;
pub const WIN_H: usize = 88;
pub const TITLE_H: usize = 10;

/// Posizione (x, y) dell'icona `i` nella griglia: 4 colonne.
pub fn icon_pos(i: usize) -> (usize, usize) {
    (8 + (i % 4) * ICON_STEP_X, BAR_H + 12 + (i / 4) * ICON_STEP_Y)
}

fn put(fb: &mut [u16], x: usize, y: usize, c: u16) {
    if x < W && y < H {
        fb[y * W + x] = c;
    }
}

fn rect(fb: &mut [u16], x: usize, y: usize, w: usize, h: usize, c: u16) {
    for yy in y..(y + h).min(H) {
        for xx in x..(x + w).min(W) {
            fb[yy * W + xx] = c;
        }
    }
}

/// Disegna il desktop completo in `fb` (W * H pixel).
pub fn paint(fb: &mut [u16]) {
    assert!(fb.len() >= W * H);
    rect(fb, 0, 0, W, H, BG);

    // Barra: nome a sinistra, due slot programma al centro, orologio a destra.
    rect(fb, 0, 0, W, BAR_H, BAR);
    rect(fb, 4, 3, 40, 6, CLOCK); // nome del sistema
    rect(fb, 92, 3, 24, 6, BAR_SLOT); // [term]
    rect(fb, 120, 3, 24, 6, BAR_SLOT); // [files]
    rect(fb, W - 30, 3, 26, 6, CLOCK); // orologio (da verificare: serve il timer)

    // Icone: files, term, giochi, notes. Riquadro chiaro = icona da aprire.
    for i in 0..4 {
        let (x, y) = icon_pos(i);
        let c = if i == 0 { ICON_HI } else { ICON };
        rect(fb, x, y, ICON_SIZE, ICON_SIZE, c);
    }

    // Finestra del terminale: titolo, corpo con bordo di 1 px, pulsante chiudi 8x8.
    rect(fb, WIN_X, WIN_Y, WIN_W, WIN_H, WIN_BORDER);
    rect(fb, WIN_X + 1, WIN_Y + 1, WIN_W - 2, TITLE_H, WIN_TITLE);
    rect(fb, WIN_X + WIN_W - 9, WIN_Y + 1, 8, 8, CLOSE);
    rect(fb, WIN_X + 1, WIN_Y + TITLE_H + 1, WIN_W - 2, WIN_H - TITLE_H - 2, WIN_BODY);
    // Prima riga del prompt, come nel concept ("$ ls").
    rect(fb, WIN_X + 4, WIN_Y + TITLE_H + 5, 10, 2, TEXT);
    put(fb, WIN_X + 1, WIN_Y + 1, WIN_BORDER);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> [u16; W * H] {
        let mut fb = [0u16; W * H];
        paint(&mut fb);
        fb
    }

    #[test]
    fn bar_is_on_top_and_background_below() {
        let fb = screen();
        assert_eq!(fb[0], BAR);
        assert_eq!(fb[(BAR_H - 1) * W + 100], BAR);
        assert_eq!(fb[(BAR_H + 2) * W + 1], BG);
    }

    #[test]
    fn icons_sit_on_the_grid() {
        let fb = screen();
        let (x, y) = icon_pos(0);
        assert_eq!(fb[(y + 1) * W + x + 1], ICON_HI);
        let (x, y) = icon_pos(1);
        assert_eq!(fb[(y + 1) * W + x + 1], ICON);
        assert_eq!(icon_pos(4).0, icon_pos(0).0); // quinta icona torna a colonna 0
    }

    #[test]
    fn window_has_title_body_and_close_button() {
        let fb = screen();
        assert_eq!(fb[(WIN_Y + 2) * W + WIN_X + 3], WIN_TITLE);
        assert_eq!(fb[(WIN_Y + 3) * W + WIN_X + WIN_W - 5], CLOSE);
        assert_eq!(fb[(WIN_Y + TITLE_H + 3) * W + WIN_X + 3], WIN_BODY);
        assert_eq!(fb[WIN_Y * W + WIN_X], WIN_BORDER);
    }

    #[test]
    fn window_fits_on_screen() {
        assert!(WIN_X + WIN_W <= W);
        assert!(WIN_Y + WIN_H <= H);
    }
}
