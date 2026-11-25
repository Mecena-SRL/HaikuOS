//! Disegno della scrivania sopra un `Canvas`. Il canvas ritaglia da solo.
use crate::layout::*;
use crate::palette::Tok::{self, T0, T1, T2, T3};
use crate::shell::{Shell, MENU_MAX};

pub trait Canvas {
    fn fill(&mut self, r: Rect, t: Tok);
    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok);
}

/// Prefisso di `s` che entra in `cols` colonne di testo (4 px per carattere).
fn clip(s: &str, cols: i32) -> &str {
    let n = (cols.max(0) as usize).min(s.len());
    s.get(..n).unwrap_or("")
}

pub fn draw<C: Canvas>(c: &mut C, s: &Shell) {
    c.fill(Rect::new(0, 0, s.w, s.h), T0);
    let active = s.wm.active();
    let ty = (BAR_H - FONT_H) / 2;

    // Pannello: menu HAIKUOS, pulsanti finestra, orologio. Sempre sopra le finestre.
    c.fill(Rect::new(0, 0, s.w, BAR_H), T3);
    let mb = menu_button();
    c.fill(mb, if s.menu { T2 } else { T3 });
    c.text(mb.x + 2, ty, "HAIKUOS", T0);
    for i in 0..s.wm.len() {
        if task_button(i).x + TASK_W > clock_x(s.w) - 2 {
            break;
        }
        let Some(w) = s.wm.get(i) else { continue };
        let tb = task_button(i);
        c.fill(tb, if Some(i) == active { T2 } else if w.min { T3 } else { T1 });
        c.text(tb.x + 2, tb.y + (tb.h - FONT_H) / 2, clip(w.title, (TASK_W - 4) / (FONT_W + 1)), T0);
        if w.min {
            c.fill(Rect::new(tb.x, tb.y + tb.h - 1, tb.w, 1), T0);
        }
    }
    c.text(clock_x(s.w), ty, core::str::from_utf8(&s.clock).unwrap_or(""), T0);

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

    // Finestre dal fondo alla cima; la attiva ha il titolo T2. Le ridotte a icona non si disegnano.
    for i in 0..s.wm.len() {
        let Some(w) = s.wm.get(i) else { continue };
        if w.min {
            continue;
        }
        let f = w.frame;
        c.fill(f, T3);
        let tb = title_bar(f);
        c.fill(tb, if Some(i) == active { T2 } else { T1 });
        let cols = (min_button(f).x - tb.x - 4) / (FONT_W + 1);
        c.text(tb.x + 2, tb.y + (TITLE_H - FONT_H) / 2, clip(w.title, cols), T0);
        c.fill(close_button(f), T0);
        c.fill(max_button(f), T0);
        c.fill(min_button(f), T0);
        c.fill(body(f), T1);
    }

    // Menu applicazioni: riga di ricerca, poi le voci che corrispondono; in cima a tutto.
    if s.menu {
        let rows = s.match_count().min(MENU_MAX);
        let r = menu_rect(rows);
        c.fill(r, T3);
        c.fill(r.inset(1), T1);
        let ry = |k: i32| BAR_H + k * MENU_ROW + (MENU_ROW - FONT_H) / 2;
        let q = if s.qlen == 0 { "CERCA" } else { s.query() };
        c.text(r.x + 3, ry(0), q, T0);
        for k in 0..rows {
            let Some(i) = s.nth_match(k) else { break };
            let row = Rect::new(r.x + 1, BAR_H + MENU_ROW * (k as i32 + 1), r.w - 2, MENU_ROW);
            if k == s.msel {
                c.fill(row, T2);
            }
            c.text(row.x + 2, ry(k as i32 + 1), clip(s.icons[i].title, (MENU_LIST_W - 6) / (FONT_W + 1)), T0);
        }
    }
}
