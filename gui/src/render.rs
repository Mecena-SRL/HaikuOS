//! Disegno della scrivania sopra un `Canvas`. Il canvas ritaglia da solo.
use crate::layout::*;
use crate::palette::Tok::{self, T0, T1, T2, T3};
use crate::shell::Shell;

pub trait Canvas {
    fn fill(&mut self, r: Rect, t: Tok);
    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok);
}

pub fn draw<C: Canvas>(c: &mut C, s: &Shell) {
    c.fill(Rect::new(0, 0, s.w, s.h), T0);

    // Barra superiore: nome del sistema e finestra attiva.
    c.fill(Rect::new(0, 0, s.w, BAR_H), T3);
    c.text(2, (BAR_H - FONT_H) / 2, "HAIKUOS", T0);
    if let Some(w) = s.wm.top() {
        c.text(2 + 8 * (FONT_W + 1), (BAR_H - FONT_H) / 2, w.title, T0);
    }

    // Icone: quadrato T2, selezionata T1, aperta con sottolineatura T2.
    let (start, count) = s.visible();
    for k in 0..count {
        let i = start + k;
        let ic = &s.icons[i];
        let cell = icon_cell(k);
        let sq = if i == s.sel { T1 } else { T2 };
        c.fill(cell, sq);
        if s.wm.find_app(ic.app).is_some() {
            c.fill(Rect::new(cell.x, cell.y + UNDERLINE_DY, ICON, 1), T2);
        }
        c.text(cell.x, cell.y + LABEL_DY, ic.label, T1);
    }

    // Finestre dal fondo alla cima; la cima ha il titolo attivo (T2).
    let n = s.wm.len();
    for i in 0..n {
        let Some(w) = s.wm.get(i) else { continue };
        let f = w.frame;
        c.fill(f, T3);
        let active = i + 1 == n;
        let tb = title_bar(f);
        c.fill(tb, if active { T2 } else { T1 });
        c.text(tb.x + 2, tb.y + (TITLE_H - FONT_H) / 2, w.title, T0);
        c.fill(close_button(f), T0);
        c.fill(body(f), T1);
    }
}
