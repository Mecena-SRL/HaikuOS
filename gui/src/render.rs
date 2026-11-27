//! Disegno della scrivania sopra un `Canvas`. Il canvas ritaglia da solo.
use crate::layout::*;
use crate::palette::Tok::{self, T0, T1, T2, T3};
use crate::apps::{num_str, FILES, LINE_H, TEXT_ROWS, THEME_NAMES, TL};
use crate::chess::{self, Outcome};

const PIECE_LETTERS: [&str; 7] = ["", "P", "N", "B", "R", "Q", "K"];
use sysapps::browser::{parse_line, Line};
use crate::shell::{Shell, MENU_MAX};
use crate::icons::{cuts, BLOCK};

pub trait Canvas {
    fn fill(&mut self, r: Rect, t: Tok);
    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok);
    /// Angoli arrotondati delle finestre: solo x86 (bibbia, sezione 6). GBA: retti.
    fn rounded(&self) -> bool {
        false
    }
    /// Grafica x86 (bibbia §3.1, §8): accenti per app, icone vere, pulsanti colorati.
    fn rich(&self) -> bool {
        false
    }
    /// Icona vera nella cella `cell` (16×16 logici = 32×32 fisici; 8×8 nei titoli). False se non disponibile.
    fn icon(&mut self, _cell: Rect, _label: &str, _app: u8) {}
}

/// Smussa i quattro angoli della finestra `f` con il fondo T0 (raggio 4 px logici).
fn round_corners<C: Canvas>(c: &mut C, f: Rect) {
    const CUT: [i32; 4] = [3, 2, 1, 0];
    for (dy, &w) in CUT.iter().enumerate() {
        let dy = dy as i32;
        if w == 0 {
            continue;
        }
        c.fill(Rect::new(f.x, f.y + dy, w, 1), T0);
        c.fill(Rect::new(f.x + f.w - w, f.y + dy, w, 1), T0);
        c.fill(Rect::new(f.x, f.y + f.h - 1 - dy, w, 1), T0);
        c.fill(Rect::new(f.x + f.w - w, f.y + f.h - 1 - dy, w, 1), T0);
    }
}

/// Prefisso di `s` che entra in `cols` colonne di testo (4 px per carattere).
fn clip(s: &str, cols: i32) -> &str {
    let n = (cols.max(0) as usize).min(s.len());
    s.get(..n).unwrap_or("")
}

/// Contenuto di una finestra: stato vero per FILE, TERM, CALC; testo statico per le altre.
fn draw_app<C: Canvas>(c: &mut C, s: &Shell, app: u8, cc: Rect) {
    let cols = cc.w / (FONT_W + 1);
    let rows = (cc.h / LINE_H).max(0) as usize;
    let row_y = |k: usize| cc.y + k as i32 * LINE_H;
    match app {
        1 if s.files.open => {
            let (_, body) = s.files_list[s.files.sel];
            for (k, line) in body.lines().take(rows).enumerate() {
                c.text(cc.x, row_y(k), clip(line, cols), T0);
            }
        }
        1 => {
            for (k, (name, _)) in s.files_list.iter().enumerate() {
                if k == s.files.sel {
                    c.fill(Rect::new(cc.x + 1, row_y(k), cc.w - 2, LINE_H), T2);
                }
                c.text(cc.x + 1, row_y(k) + 1, clip(name, cols), T0);
            }
        }
        2 => {
            // Ultime righe di storico, poi il prompt con il cursore.
            let t = &s.term;
            let total = t.hn + 1;
            for (k, idx) in (total.saturating_sub(rows)..total).enumerate() {
                if idx < t.hn {
                    let line = core::str::from_utf8(&t.hist[idx][..t.hl[idx]]).unwrap_or("");
                    c.text(cc.x, row_y(k), clip(line, cols), T0);
                } else {
                    // Prompt ": " e cursore come blocco T2 (il font non ha il trattino basso).
                    let mut p = [0u8; TL + 2];
                    p[..2].copy_from_slice(b": ");
                    p[2..2 + t.len].copy_from_slice(&t.line[..t.len]);
                    let line = core::str::from_utf8(&p[..2 + t.len]).unwrap_or("");
                    c.text(cc.x, row_y(k), clip(line, cols), T0);
                    let cx = cc.x + (2 + t.len as i32) * (FONT_W + 1);
                    if cx + FONT_W <= cc.x + cc.w {
                        c.fill(Rect::new(cx, row_y(k), FONT_W, FONT_H), T2);
                    }
                }
            }
        }
        3 => {
            let mut buf = [0u8; 21];
            let shown = s.calc.show(&mut buf);
            c.text(cc.x, row_y(0), shown, T0);
            c.text(cc.x, row_y(2), clip("0-9 + - * / = C", cols), T0);
        }
        4 => {
            // Pagina dalla riga di scorrimento; i collegamenti con la selezione in T2.
            if let Some(body) = s.browser.current() {
                let (mut link, mut row) = (0usize, 0usize);
                for (i, line) in body.lines().enumerate() {
                    let is_link = matches!(parse_line(line), Line::Link { .. });
                    if i < s.bscroll {
                        link += is_link as usize;
                        continue;
                    }
                    if row >= TEXT_ROWS.min(rows) {
                        break;
                    }
                    let y = row_y(row);
                    row += 1;
                    match parse_line(line) {
                        Line::Heading(t) | Line::Text(t) => c.text(cc.x, y, clip(t, cols), T0),
                        Line::Link { label, .. } => {
                            if link == s.bsel {
                                c.fill(Rect::new(cc.x, y, cc.w, LINE_H), T2);
                            }
                            c.text(cc.x + 1, y + 1, clip(label, cols - 1), T0);
                            link += 1;
                        }
                    }
                }
            }
        }
        5 | 9 => {
            // Buffer dell'editor (5) o dell'IDE (9), cursore T2, e in fondo il pannello dell'uscita di F5.
            let ed = if app == 9 { &s.ide } else { &s.editor };
            // IDE: prima riga il nome del file (L lo cambia), il testo sotto.
            let off = if app == 9 { 1 } else { 0 };
            if app == 9 {
                c.text(cc.x, row_y(0), clip(FILES[s.ide_sel].0, cols), T0);
            }
            let code = ed.lines().min(7 - off);
            for i in 0..code {
                c.text(cc.x, row_y(i + off), clip(ed.line(i), cols), T0);
            }
            let (r, col) = ed.cursor();
            let cx = cc.x + col as i32 * (FONT_W + 1);
            if r < code && cx + FONT_W <= cc.x + cc.w {
                c.fill(Rect::new(cx, row_y(r + off), FONT_W, FONT_H), T2);
            }
            if rows >= TEXT_ROWS {
                c.fill(Rect::new(cc.x, row_y(7), cc.w, 1), T3);
                let o = &s.eout;
                for k in 0..o.hn.min(2) {
                    let idx = o.hn - o.hn.min(2) + k;
                    let line = core::str::from_utf8(&o.hist[idx][..o.hl[idx]]).unwrap_or("");
                    c.text(cc.x, row_y(8 + k), clip(line, cols), T0);
                }
            }
        }
        6 => {
            c.text(cc.x, row_y(0), "TEMA", T0);
            c.text(cc.x, row_y(1), clip(THEME_NAMES[s.theme as usize % 3], cols), T0);
            c.text(cc.x, row_y(3), clip("INVIO CAMBIA", cols), T0);
        }
        7 => {
            c.text(cc.x, row_y(0), core::str::from_utf8(&s.clock).unwrap_or(""), T0);
            c.text(cc.x, row_y(2), clip("EUROPA ROMA", cols), T0);
        }
        8 => {
            // Lettore: nome del file scelto (←/→), poi il testo con scorrimento (↑/↓).
            let (name, body) = s.files_list[s.files.sel];
            c.text(cc.x, row_y(0), clip(name, cols), T0);
            c.fill(Rect::new(cc.x, row_y(1), cc.w, 1), T3);
            for (k, line) in body.lines().skip(s.rscroll).take(rows.min(TEXT_ROWS) - 2).enumerate() {
                c.text(cc.x, row_y(k + 2), clip(line, cols), T0);
            }
        }
        10 => {
            // Documento: nome e posizione (←/→), poi il testo con scorrimento (↑/↓).
            const NAMES: [&str; 4] = ["DOCX", "PDF", "PPTX", "XLSX"];
            c.text(cc.x, row_y(0), clip(NAMES[s.doc_sel], cols), T0);
            c.fill(Rect::new(cc.x, row_y(1), cc.w, 1), T3);
            let text = s.docs[s.doc_sel];
            for (k, line) in text.split(|&b| b == b'\n').skip(s.rscroll).take(rows.min(TEXT_ROWS) - 2).enumerate() {
                c.text(cc.x, row_y(k + 2), clip(core::str::from_utf8(line).unwrap_or(""), cols), T0);
            }
        }
        11 => {
            // Scacchi: scacchiera 8×8 (cursore in T2, case scure in T3), poi lo stato.
            const CW: i32 = 10;
            const CH: i32 = 8;
            let (ox, oy) = (cc.x + 4, cc.y + 2);
            for r in 0..8u8 {
                for f in 0..8u8 {
                    let sq = (7 - r) * 8 + f;
                    let (x, y) = (ox + f as i32 * CW, oy + r as i32 * CH);
                    if ((7 - r) + f) % 2 == 0 {
                        c.fill(Rect::new(x, y, CW, CH), T3);
                    }
                    if sq == s.chess.cur {
                        c.fill(Rect::new(x, y, CW, CH), T2);
                    }
                    let p = s.board.sq[sq as usize];
                    if p != 0 {
                        let ink = if p >> 3 == chess::WHITE { T0 } else { T2 };
                        c.text(x + 3, y + 1, PIECE_LETTERS[(p & 7) as usize], ink);
                    }
                }
            }
            let msg = match s.board.outcome() {
                Outcome::Checkmate(w) if w == chess::WHITE => "BIANCO HA VINTO",
                Outcome::Checkmate(_) => "NERO HA VINTO",
                Outcome::Stalemate => "PATTA",
                Outcome::Check => "SCACCO",
                Outcome::Ongoing if s.board.turn == chess::WHITE => "TOCCA AL BIANCO",
                Outcome::Ongoing => "TOCCA AL NERO",
            };
            c.text(cc.x + 4, oy + 8 * CH + 3, clip(msg, cols), T0);
        }
        _ => {}
    }
}

/// Sfondo a colline (scelta di Ivan): onde triangolari in T2 dal 70% in giù.
fn hills<C: Canvas>(c: &mut C, w: i32, h: i32) {
    let (base, amp, period) = (h * 7 / 10, h / 8, (w / 2).max(2));
    for x in 0..w {
        let t = x % period;
        let tri = if t * 2 < period { t * 2 } else { (period - t) * 2 };
        let top = base - amp * tri / period;
        c.fill(Rect::new(x, top, 1, h - top), T2);
    }
}

/// Puntatore a freccia: riempimento T1 con bordo T0 (bibbia §13).
pub fn pointer<C: Canvas>(c: &mut C, x: i32, y: i32) {
    for k in 0..12 {
        c.fill(Rect::new(x, y + k, 2 + k * 2 / 3, 1), T0);
    }
    for k in 0..10 {
        let w = k * 2 / 3;
        if w > 0 {
            c.fill(Rect::new(x + 1, y + k + 1, w, 1), T1);
        }
    }
}

/// Banner di notifica sotto la barra, centrato, T2 con testo T0 (bibbia §13).
fn notice<C: Canvas>(c: &mut C, s: &Shell) {
    let msg = s.notice();
    if msg.is_empty() {
        return;
    }
    let w = msg.len() as i32 * (FONT_W + 1) + 12;
    let x = (s.w - w) / 2;
    c.fill(Rect::new(x, BAR_H + 2, w, FONT_H + 4), T2);
    c.text(x + 6, BAR_H + 4, msg, T0);
}

/// Smussa gli angoli di un pulsante 8×8 con un pixel T0 (solo x86, bibbia §13).
fn corners<C: Canvas>(c: &mut C, r: Rect) {
    if c.rounded() {
        for (x, y) in [(r.x, r.y), (r.x + r.w - 1, r.y), (r.x, r.y + r.h - 1), (r.x + r.w - 1, r.y + r.h - 1)] {
            c.fill(Rect::new(x, y, 1, 1), T0);
        }
    }
}

pub fn draw<C: Canvas>(c: &mut C, s: &Shell) {
    c.fill(Rect::new(0, 0, s.w, s.h), T0);
    hills(c, s.w, s.h);
    // Sfondo: marchio "HAIKUOS" in basso a destra, in T3 sopra le colline.
    c.text(s.w - 7 * (FONT_W + 1) - 2, s.h - FONT_H - 2, "HAIKUOS", T3);
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
        c.fill(tb, if Some(i) == active { if c.rich() { Tok::A(w.app) } else { T2 } } else if w.min { T3 } else { T1 });
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
        if c.rich() {
            if i == s.sel {
                c.fill(cell.inset(-1), T1);
            }
            c.icon(cell, ic.label, ic.app);
        } else {
            if i == s.sel {
                c.fill(cell, T1);
            } else {
                let (bx, by, bw, bh) = BLOCK;
                c.fill(Rect::new(cell.x + bx, cell.y + by, bw, bh), T2);
            }
            for &(x, y, w, h) in cuts(ic.label) {
                c.fill(Rect::new(cell.x + x, cell.y + y, w, h), T0);
            }
        }
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
        let rich = c.rich();
        // x86: titolo dell'app nel suo colore (attiva) con icona 8×8 a sinistra; GBA: T2/T1 come da bibbia.
        let tx = if rich {
            c.fill(tb, if Some(i) == active { Tok::A(w.app) } else { T1 });
            let label = s.icons.iter().find(|ic| ic.app == w.app).map_or("", |ic| ic.label);
            c.icon(Rect::new(tb.x + 1, tb.y + 1, TITLE_H - 2, TITLE_H - 2), label, w.app);
            tb.x + TITLE_H
        } else {
            c.fill(tb, if Some(i) == active { T2 } else { T1 });
            tb.x + 2
        };
        let cols = (min_button(f).x - tx - 4) / (FONT_W + 1);
        c.text(tx, tb.y + (TITLE_H - FONT_H) / 2, clip(w.title, cols), T0);
        let (cl, mx, mn) = if rich { (Tok::A(11), Tok::A(10), Tok::A(9)) } else { (T0, T0, T0) };
        c.fill(close_button(f), cl);
        c.fill(max_button(f), mx);
        c.fill(min_button(f), mn);
        for r in [close_button(f), max_button(f), min_button(f)] {
            corners(c, r);
        }
        c.fill(body(f), T1);
        draw_app(c, s, w.app, content(f));
        if c.rounded() {
            round_corners(c, f);
        }
    }

    notice(c, s);
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
