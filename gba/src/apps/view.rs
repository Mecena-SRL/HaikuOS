//! Disegno delle app di sistema nel corpo della finestra (`x0`, `y0` = origine del corpo, 106×76).
//! Colori dalla palette del tema: `p[0]` testo, `p[1]` corpo, `p[2]` evidenza.

use crate::apps::{clock::Clock, notes::Notes, settings::Settings};
use crate::font::{self, ADVANCE};

const BODY_W: usize = 106;
const BODY_H: usize = 76;
const COLS: usize = 24;
const ROWS: usize = 10;
const LINE: usize = 6;
const TOP: usize = 10;

fn fill(fb: &mut [u16], w: usize, h: usize, x: usize, y: usize, rw: usize, rh: usize, c: u16) {
    for yy in y..(y + rh).min(h) {
        for xx in x..(x + rw).min(w) {
            fb[yy * w + xx] = c;
        }
    }
}

fn digit(n: usize) -> u8 {
    b'0' + (n % 10) as u8
}

fn body(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, p: [u16; 4]) {
    fill(fb, w, h, x0, y0, BODY_W, BODY_H, p[1]);
}

/// Note: intestazione "NOTA n DI m", poi il testo a capo, con l'ultima riga in fondo.
pub fn notes(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, n: &Notes, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    if n.count() == 0 {
        font::draw(fb, w, h, x0 + 2, y0 + 2, b"NESSUNA NOTA", p[0]);
        return;
    }
    let head = [b'N', b'O', b'T', b'A', b' ', digit(n.selected() + 1), b' ', b'D', b'I', b' ', digit(n.count())];
    font::draw(fb, w, h, x0 + 2, y0 + 2, &head, p[0]);

    let text = n.testo();
    let len = text.len();
    let start = (len / COLS + 1).saturating_sub(ROWS);
    for i in 0..=len {
        let (c, color) = if i < len { (text[i], p[0]) } else { (n.pending(), p[2]) };
        let line = i / COLS;
        if line < start {
            continue;
        }
        let x = x0 + 2 + (i % COLS) * ADVANCE;
        let y = y0 + TOP + (line - start) * LINE;
        font::draw(fb, w, h, x, y, &[c], color);
    }
}

/// Orologio: ora grande al centro.
pub fn clock(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, c: &Clock, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    font::draw(fb, w, h, x0 + 43, y0 + 30, &c.testo(), p[0]);
}

/// Impostazioni: numero del tema, quattro campioni della palette, nome del sistema.
pub fn settings(fb: &mut [u16], w: usize, h: usize, x0: usize, y0: usize, s: &Settings, p: [u16; 4]) {
    body(fb, w, h, x0, y0, p);
    let head = [b'T', b'E', b'M', b'A', b' ', digit(s.theme() + 1)];
    font::draw(fb, w, h, x0 + 2, y0 + 2, &head, p[0]);
    let pal = s.palette();
    for (i, &c) in pal.iter().enumerate() {
        fill(fb, w, h, x0 + 3 + i * 24, y0 + 17, 22, 16, p[0]);
        fill(fb, w, h, x0 + 4 + i * 24, y0 + 18, 20, 14, c);
    }
    font::draw(fb, w, h, x0 + 2, y0 + 42, b"NOME", p[0]);
    font::draw(fb, w, h, x0 + 2, y0 + 52, s.nome(), p[0]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::settings::PALETTES;

    const W: usize = 240;
    const H: usize = 160;

    #[test]
    fn impostazioni_mostra_la_palette_del_tema() {
        let mut s = Settings::new();
        s.next_theme();
        let mut fb = vec![0u16; W * H];
        settings(&mut fb, W, H, 10, 10, &s, s.palette());
        for i in 0..4 {
            assert_eq!(fb[(10 + 18 + 2) * W + 10 + 4 + i * 24 + 2], PALETTES[1][i]);
        }
    }

    #[test]
    fn nota_evidenzia_il_carattere_in_attesa() {
        let p = [1u16, 31, 0x7C1F, 3];
        let mut n = Notes::new();
        n.nuova();
        let mut fb = vec![0u16; W * H];
        notes(&mut fb, W, H, 10, 10, &n, p);
        let nel_riquadro = |fb: &[u16]| {
            (0..5).any(|dy| (0..3).any(|dx| fb[(10 + TOP + dy) * W + 12 + dx] == p[2]))
        };
        assert!(!nel_riquadro(&fb)); // spazio in attesa: niente da evidenziare
        n.up(); // 'A'
        let mut fb = vec![0u16; W * H];
        notes(&mut fb, W, H, 10, 10, &n, p);
        assert!(nel_riquadro(&fb));
    }

    #[test]
    fn orologio_disegna_il_testo() {
        let p = [1u16, 31, 0x7C1F, 3];
        let mut fb = vec![0u16; W * H];
        clock(&mut fb, W, H, 10, 10, &Clock::new(9, 5), p);
        assert!(fb.iter().any(|&px| px == p[0]));
        assert_eq!(fb[(10 + 76 - 2) * W + 10 + 100], p[1]); // corpo
    }
}
