//! Schermata del desktop dal concept (docs/desktop-concept.md, sezioni 2 e 3).
//! Tema scuro e freddo. Disegna in un framebuffer 240x160, senza dipendenze da hardware.
#![allow(dead_code)]

pub const W: usize = 240;
pub const H: usize = 160;

const fn rgb(r: u16, g: u16, b: u16) -> u16 {
    r | (g << 5) | (b << 10)
}

// Quattro tinte (docs/desktop-concept.md, sezione 1): T0 sfondo, T1 bianco, T2 arancio, T3 blu.
pub const T0: u16 = rgb(1, 2, 6);
pub const T1: u16 = rgb(31, 31, 31);
pub const T2: u16 = rgb(31, 16, 0);
pub const T3: u16 = rgb(0, 20, 31);

pub const BG: u16 = T0;
pub const BAR: u16 = T3;
pub const BAR_SLOT: u16 = T2;
pub const CLOCK: u16 = T1;
pub const ICON: u16 = T2;
pub const ICON_HI: u16 = T1;
pub const WIN_TITLE: u16 = T2;
pub const WIN_BODY: u16 = T1;
pub const WIN_BORDER: u16 = T0;
pub const CLOSE: u16 = T1;
pub const TEXT: u16 = T0;

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
/// Numero di icone sulla riga (4 colonne, una sola riga nel primo schizzo).
pub const ICONS: usize = 4;
/// Icona che apre la finestra del terminale.
pub const TERM_ICON: usize = 1;

pub const KEY_A: u16 = 1 << 0;
pub const KEY_B: u16 = 1 << 1;
pub const KEY_RIGHT: u16 = 1 << 4;
pub const KEY_LEFT: u16 = 1 << 5;
pub const KEY_UP: u16 = 1 << 6;
pub const KEY_DOWN: u16 = 1 << 7;

/// Stato del desktop: icona selezionata e finestra del terminale aperta o no.
pub struct Desktop {
    sel: usize,
    win_open: bool,
    prev: u16,
}

impl Desktop {
    pub const fn new() -> Self {
        Self { sel: 0, win_open: false, prev: 0 }
    }

    pub fn sel(&self) -> usize {
        self.sel
    }

    pub fn win_open(&self) -> bool {
        self.win_open
    }

    /// Un passo di input (tasti premuti in questo frame). Ritorna true se il desktop va ridisegnato.
    pub fn step(&mut self, keys: u16) -> bool {
        let pressed = keys & !self.prev;
        self.prev = keys;
        let before = (self.sel, self.win_open);
        if pressed & (KEY_RIGHT | KEY_DOWN) != 0 {
            self.sel = (self.sel + 1) % ICONS;
        }
        if pressed & (KEY_LEFT | KEY_UP) != 0 {
            self.sel = (self.sel + ICONS - 1) % ICONS;
        }
        if pressed & KEY_A != 0 && self.sel == TERM_ICON {
            self.win_open = true;
        }
        if pressed & KEY_B != 0 {
            self.win_open = false;
        }
        before != (self.sel, self.win_open)
    }
}

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
pub fn paint(fb: &mut [u16], d: &Desktop) {
    assert!(fb.len() >= W * H);
    rect(fb, 0, 0, W, H, BG);

    // Barra: nome a sinistra, due slot programma al centro, orologio a destra.
    rect(fb, 0, 0, W, BAR_H, BAR);
    rect(fb, 4, 3, 40, 6, CLOCK); // nome del sistema
    rect(fb, 92, 3, 24, 6, BAR_SLOT); // [term]
    rect(fb, 120, 3, 24, 6, BAR_SLOT); // [files]
    rect(fb, W - 30, 3, 26, 6, CLOCK); // orologio (da verificare: serve il timer)

    // Icone: files, term, giochi, notes. Riquadro chiaro = icona da aprire.
    for i in 0..ICONS {
        let (x, y) = icon_pos(i);
        let c = if i == d.sel() { ICON_HI } else { ICON };
        rect(fb, x, y, ICON_SIZE, ICON_SIZE, c);
    }
    if !d.win_open() {
        return;
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

    fn screen(d: &Desktop) -> [u16; W * H] {
        let mut fb = [0u16; W * H];
        paint(&mut fb, d);
        fb
    }

    #[test]
    fn navigation_wraps_and_terminal_opens_and_closes() {
        let mut d = Desktop::new();
        assert!(d.step(KEY_RIGHT));
        assert_eq!(d.sel(), 1);
        assert!(!d.step(KEY_RIGHT)); // tasto ancora premuto: nessun passo in più
        d.step(0);
        assert!(d.step(KEY_RIGHT));
        assert_eq!(d.sel(), 2);
        d.step(0);
        assert!(!d.step(KEY_A)); // icona 2 non apre nulla
        assert!(!d.win_open());
        d.step(0);
        d.step(KEY_LEFT);
        assert_eq!(d.sel(), TERM_ICON);
        d.step(0);
        assert!(d.step(KEY_A));
        assert!(d.win_open());
        d.step(0);
        assert!(d.step(KEY_B));
        assert!(!d.win_open());
        d.step(0);
        d.step(KEY_UP);
        assert_eq!(d.sel(), 0);
        d.step(0);
        d.step(KEY_LEFT);
        assert_eq!(d.sel(), ICONS - 1);
    }

    #[test]
    fn bar_is_on_top_and_background_below() {
        let fb = screen(&Desktop::new());
        assert_eq!(fb[0], BAR);
        assert_eq!(fb[(BAR_H - 1) * W + 100], BAR);
        assert_eq!(fb[(BAR_H + 2) * W + 1], BG);
    }

    #[test]
    fn icons_sit_on_the_grid() {
        let fb = screen(&Desktop::new());
        let (x, y) = icon_pos(0);
        assert_eq!(fb[(y + 1) * W + x + 1], ICON_HI);
        let (x, y) = icon_pos(1);
        assert_eq!(fb[(y + 1) * W + x + 1], ICON);
        assert_eq!(icon_pos(4).0, icon_pos(0).0); // quinta icona torna a colonna 0
    }

    #[test]
    fn window_has_title_body_and_close_button() {
        let mut d = Desktop::new();
        d.step(KEY_RIGHT); // icona del terminale
        d.step(0);
        d.step(KEY_A);
        let fb = screen(&d);
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
