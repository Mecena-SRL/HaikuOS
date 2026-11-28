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
    /// Rettangolo con angoli di raggio `radius`. Di base è pieno: i backend con angoli lo ridefiniscono.
    fn rect_rounded(&mut self, r: Rect, _radius: i32, t: Tok) {
        self.fill(r, t);
    }
    /// Icona vettoriale: rettangoli `(x, y, w, h)` su una griglia 16×16, scalati nella cella `cell`.
    fn icon_vec(&mut self, cell: Rect, rects: &[(i32, i32, i32, i32)], t: Tok) {
        for &(x, y, w, h) in rects {
            let px = cell.x + x * cell.w / 16;
            let py = cell.y + y * cell.h / 16;
            self.fill(Rect::new(px, py, (w * cell.w / 16).max(1), (h * cell.h / 16).max(1)), t);
        }
    }
    /// Menu a righe nel rettangolo `r`: la riga `sel` è evidenziata. Ogni riga è alta `MENU_ROW`.
    fn menu(&mut self, r: Rect, rows: &[&str], sel: usize) {
        self.fill(r, T0);
        for (i, label) in rows.iter().enumerate() {
            let y = r.y + i as i32 * MENU_ROW;
            if y + MENU_ROW > r.y + r.h {
                break;
            }
            let (bg, fg) = if i == sel { (T2, T0) } else { (T0, T2) };
            self.fill(Rect::new(r.x, y, r.w, MENU_ROW), bg);
            self.text(r.x + 2, y + (MENU_ROW - FONT_H) / 2, label, fg);
        }
    }
    /// Pixel con copertura `a` (0..=255) del token `t`, per l'antialiasing. Di base: soglia a metà (pixel pieno).
    /// Il backend a 32 bit può ridefinirlo per mescolare col fondo.
    fn blend(&mut self, x: i32, y: i32, a: u8, t: Tok) {
        if a >= 128 {
            self.fill(Rect::new(x, y, 1, 1), t);
        }
    }
    /// Backend con primitive antialiasate (angoli, cerchi, linee, poligoni): le cornici non usano pixel d'angolo a mano.
    fn smooth(&self) -> bool {
        false
    }
    /// Poligono pieno a regola pari, vertici in ordine (px logici). Di base: righe piene, senza antialiasing.
    fn poly(&mut self, pts: &[(i32, i32)], t: Tok) {
        let Some(y0) = pts.iter().map(|p| p.1).min() else { return };
        let y1 = pts.iter().map(|p| p.1).max().unwrap_or(y0);
        for y in y0..y1 {
            let mut xs = [0i32; 16];
            let mut n = 0;
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                if (a.1 > y) != (b.1 > y) && n < xs.len() {
                    xs[n] = a.0 + (y - a.1) * (b.0 - a.0) / (b.1 - a.1);
                    n += 1;
                }
            }
            xs[..n].sort_unstable();
            for k in (0..n / 2 * 2).step_by(2) {
                if xs[k + 1] > xs[k] {
                    self.fill(Rect::new(xs[k], y, xs[k + 1] - xs[k], 1), t);
                }
            }
        }
    }
    /// Testo vettoriale con antialiasing (`vfont`): `size` px per em. Ritorna la larghezza in px.
    fn vtext(&mut self, x: i32, y: i32, size: i32, s: &str, t: Tok) -> i32
    where
        Self: Sized,
    {
        crate::vfont::draw_text(self, x, y, size, s, t)
    }
    /// Segmento di spessore `w` (px, minimo 1), da `(x1, y1)` a `(x2, y2)`. Di base: quadrati pieni lungo Bresenham.
    fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, w: i32, t: Tok) {
        let w = w.max(1);
        let (dx, dy) = ((x2 - x1).abs(), -(y2 - y1).abs());
        let (sx, sy) = (if x1 < x2 { 1 } else { -1 }, if y1 < y2 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x1, y1, dx + dy);
        loop {
            self.fill(Rect::new(x - w / 2, y - w / 2, w, w), t);
            if x == x2 && y == y2 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }
}

/// Raggio degli angoli delle finestre e dei pulsanti circolari (px logici).
const CORNER: i32 = 4;

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

/// Tasti della calcolatrice, 4 colonne × 5 righe; `""` è cella vuota. Solo etichette: la logica è in `Calc`.
const CALC_KEYS: [&str; 20] = [
    "7", "8", "9", "/", //
    "4", "5", "6", "*", //
    "1", "2", "3", "-", //
    "0", ".", "=", "+", //
    "C", "", "", "",
];

/// Direzione di una lancetta in gradi (0 = ore 12, senso orario), come `(sin, -cos)` ×1000.
/// Seno di Bhaskara I (errore < 0,2%): solo interi, nessuna tabella.
fn hand(deg: i32) -> (i32, i32) {
    fn sin(d: i32) -> i32 {
        let d = d.rem_euclid(360);
        if d >= 180 {
            return -sin(d - 180);
        }
        let p = d * (180 - d);
        16 * p * 1000 / (40500 - p)
    }
    (sin(deg), -sin(deg + 90))
}

/// Prefisso di `s` che entra in `cols` colonne di testo (4 px per carattere).
fn clip(s: &str, cols: i32) -> &str {
    let n = (cols.max(0) as usize).min(s.len());
    s.get(..n).unwrap_or("")
}

/// Larghezza in px di `s`: vettoriale sui backend ricchi, bitmap (3×5 o 5×7) altrove.
fn text_w(rich: bool, s: &str) -> i32 {
    if rich {
        crate::vfont::advance(LINE_H - 1, s)
    } else {
        s.len() as i32 * (FONT_W + 1)
    }
}

/// Riga di testo in `x` entro `w` px: `vtext` sui backend ricchi, bitmap altrove (GBA invariato).
fn label<C: Canvas>(c: &mut C, x: i32, y: i32, w: i32, s: &str, t: Tok, cols: i32) {
    if c.rich() {
        let mut n = s.len();
        while n > 0 && (!s.is_char_boundary(n) || text_w(true, &s[..n]) > w) {
            n -= 1;
        }
        c.vtext(x, y, LINE_H - 1, &s[..n], t);
    } else {
        c.text(x, y, clip(s, cols), t);
    }
}

/// Icona vettoriale di un file (griglia 16×16 di `icon_vec`).
const DOC_ICON: &[(i32, i32, i32, i32)] = &[(3, 1, 10, 14)];
const DOC_LINES: &[(i32, i32, i32, i32)] = &[(5, 5, 6, 1), (5, 8, 6, 1), (5, 11, 4, 1)];

/// Contenuto di una finestra: stato vero per FILE, TERM, CALC; testo statico per le altre.
fn draw_app<C: Canvas>(c: &mut C, s: &Shell, app: u8, cc: Rect) {
    let cols = cc.w / (FONT_W + 1);
    let rows = (cc.h / LINE_H).max(0) as usize;
    let row_y = |k: usize| cc.y + k as i32 * LINE_H;
    match app {
        1 if s.files.open => {
            let (_, body) = s.files_list[s.files.sel];
            for (k, line) in body.lines().take(rows).enumerate() {
                label(c, cc.x, row_y(k), cc.w, line, T0, cols);
            }
        }
        1 => {
            for (k, (name, _)) in s.files_list.iter().enumerate() {
                if k == s.files.sel {
                    c.rect_rounded(Rect::new(cc.x + 1, row_y(k), cc.w - 2, LINE_H), 2, T2);
                }
                let mut x = cc.x + 1;
                if c.rich() {
                    let cell = Rect::new(x + 1, row_y(k), LINE_H - 2, LINE_H - 2);
                    c.icon_vec(cell, DOC_ICON, T1);
                    c.icon_vec(cell, DOC_LINES, T0);
                    x = cell.x + cell.w + 2;
                }
                label(c, x, row_y(k) + 1, cc.x + cc.w - x, name, T0, cols);
            }
        }
        2 => {
            // Ultime righe di storico, poi il prompt con il cursore.
            let t = &s.term;
            let total = t.hn + 1;
            for (k, idx) in (total.saturating_sub(rows)..total).enumerate() {
                if idx < t.hn {
                    let line = core::str::from_utf8(&t.hist[idx][..t.hl[idx]]).unwrap_or("");
                    label(c, cc.x, row_y(k), cc.w, line, T0, cols);
                } else {
                    // Prompt ": " e cursore come blocco T2 (il font non ha il trattino basso).
                    let mut p = [0u8; TL + 2];
                    p[..2].copy_from_slice(b": ");
                    p[2..2 + t.len].copy_from_slice(&t.line[..t.len]);
                    let line = core::str::from_utf8(&p[..2 + t.len]).unwrap_or("");
                    label(c, cc.x, row_y(k), cc.w, line, T0, cols);
                    let cx = cc.x + text_w(c.rich(), line);
                    if cx + FONT_W <= cc.x + cc.w {
                        c.rect_rounded(Rect::new(cx, row_y(k), FONT_W, FONT_H), 1, T2);
                    }
                }
            }
        }
        3 => {
            // Display T2 con cifre T0; tasti 4×5 arrotondati (T1), etichetta vettoriale.
            let mut buf = [0u8; 21];
            let shown = s.calc.show(&mut buf);
            let disp = Rect::new(cc.x, cc.y, cc.w, 10);
            c.rect_rounded(disp, 2, T0);
            c.vtext(disp.x + 2, disp.y + 1, 8, shown, T1);
            let gy = disp.y + disp.h + 2;
            let (gap, kw) = (1, (cc.w - 3) / 4);
            let kh = ((cc.y + cc.h - gy - 4) / 5).max(4);
            for (i, key) in CALC_KEYS.iter().enumerate() {
                if key.is_empty() {
                    continue;
                }
                let (col, row) = ((i % 4) as i32, (i / 4) as i32);
                let k = Rect::new(cc.x + col * (kw + gap), gy + row * (kh + gap), kw, kh - gap);
                c.rect_rounded(k, 2, T2);
                c.vtext(k.x + kw / 2 - 2, k.y + (k.h - kh / 2) / 2, (kh - 2).clamp(3, 8), key, T1);
            }
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
                label(c, cc.x, row_y(i + off), cc.w, ed.line(i), T0, cols);
            }
            let (r, col) = ed.cursor();
            if r < code {
                let line = ed.line(r);
                let cx = cc.x + text_w(c.rich(), line.get(..col).unwrap_or(line));
                if cx + FONT_W <= cc.x + cc.w {
                    c.rect_rounded(Rect::new(cx, row_y(r + off), FONT_W, FONT_H), 1, T2);
                }
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
            // Titolo vettoriale, poi la scelta del tema come menu (riga selezionata in T2).
            c.vtext(cc.x, cc.y, 7, "TEMA", T2);
            c.menu(Rect::new(cc.x, cc.y + 9, cc.w, MENU_ROW * THEME_NAMES.len() as i32), &THEME_NAMES, s.theme as usize % 3);
        }
        7 => {
            // Quadrante analogico (ore e minuti) a sinistra, orario digitale a destra.
            let b = &s.clock;
            let (h, m) = ((b[0] - b'0') as i32 * 10 + (b[1] - b'0') as i32, (b[3] - b'0') as i32 * 10 + (b[4] - b'0') as i32);
            let d = cc.h.min(cc.w * 2 / 3);
            let face = Rect::new(cc.x, cc.y, d, d);
            c.rect_rounded(face, d / 3, T0);
            let (cx, cy, r) = (face.x + d / 2, face.y + d / 2, d / 2 - 2);
            for k in 0..12 {
                let (dx, dy) = hand(k * 30);
                c.line(cx + dx * r / 1000, cy + dy * r / 1000, cx + dx * (r - 2) / 1000, cy + dy * (r - 2) / 1000, 1, T2);
            }
            let (dx, dy) = hand((h % 12 * 60 + m) / 2);
            c.line(cx, cy, cx + dx * r / 2000, cy + dy * r / 2000, 2, T2);
            let (dx, dy) = hand(m * 6);
            c.line(cx, cy, cx + dx * r * 9 / 10000, cy + dy * r * 9 / 10000, 1, T2);
            let tx = face.x + d + 3;
            c.vtext(tx, cc.y + cc.h / 2 - 6, 7, core::str::from_utf8(b).unwrap_or(""), T2);
            c.vtext(tx, cc.y + cc.h / 2 + 6, 5, "EUROPA ROMA", T2);
        }
        8 => {
            // Lettore: nome del file scelto (←/→), poi il testo con scorrimento (↑/↓).
            let (name, body) = s.files_list[s.files.sel];
            label(c, cc.x, row_y(0), cc.w, name, T0, cols);
            c.fill(Rect::new(cc.x, row_y(1), cc.w, 1), T3);
            for (k, line) in body.lines().skip(s.rscroll).take(rows.min(TEXT_ROWS) - 2).enumerate() {
                label(c, cc.x, row_y(k + 2), cc.w, line, T0, cols);
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
    if c.rich() {
        // Vettoriale: un poligono con picchi e valli ogni `period / 2`, chiuso in basso.
        let q = period / 2;
        let pts = [(0, base), (q, base - amp), (2 * q, base), (3 * q, base - amp), (w, base), (w, h), (0, h)];
        c.poly(&pts, T2);
        return;
    }
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

/// Simbolo del pulsante 8×8: `-` (riduci), `+` (massimizza), `x` (chiudi). Bibbia §13.
fn glyph<C: Canvas>(c: &mut C, r: Rect, sym: char, col: Tok) {
    if c.smooth() {
        // `-` e `+` sono barre piene da 2 px centrate (come nel ramo pixel): nitide e senza sbavature.
        // `x` è un tratto antialiasato con estremi simmetrici sul centro del pulsante.
        let (x0, x1) = (r.x + 1, r.x + r.w - 2);
        let (y0, y1) = (r.y + 1, r.y + r.h - 2);
        match sym {
            '-' => c.fill(Rect::new(r.x + 1, r.y + 3, 6, 2), col),
            '+' => {
                c.fill(Rect::new(r.x + 1, r.y + 3, 6, 2), col);
                c.fill(Rect::new(r.x + 3, r.y + 1, 2, 6), col);
            }
            _ => {
                c.line(x0, y0, x1, y1, 1, col);
                c.line(x1, y0, x0, y1, 1, col);
            }
        }
        return;
    }
    match sym {
        '-' => c.fill(Rect::new(r.x + 1, r.y + 3, 6, 2), col),
        '+' => {
            c.fill(Rect::new(r.x + 1, r.y + 3, 6, 2), col);
            c.fill(Rect::new(r.x + 3, r.y + 1, 2, 6), col);
        }
        _ => {
            for i in 1..7 {
                c.fill(Rect::new(r.x + i, r.y + i, 1, 1), col);
                c.fill(Rect::new(r.x + 7 - i, r.y + i, 1, 1), col);
            }
        }
    }
}

/// Ombra morbida sotto la finestra (solo x86, bibbia §13): un bordo T0 di 2 px fuori dal bordo.
fn shadow<C: Canvas>(c: &mut C, f: Rect) {
    if c.rounded() {
        c.rect_rounded(Rect::new(f.x + 2, f.y + 2, f.w, f.h), CORNER, T0);
    }
}

/// Smussa gli angoli di un pulsante 8×8 con un pixel T0 (solo x86, bibbia §13).
fn corners<C: Canvas>(c: &mut C, r: Rect) {
    if c.rounded() && !c.smooth() {
        for (x, y) in [(r.x, r.y), (r.x + r.w - 1, r.y), (r.x, r.y + r.h - 1), (r.x + r.w - 1, r.y + r.h - 1)] {
            c.fill(Rect::new(x, y, 1, 1), T0);
        }
    }
}

/// Movimento di un frame: `win_dy[i]` sposta in basso la finestra `i` (px logici);
/// `sel` è il selettore in volo fra due icone (scorrimento reattivo).
#[derive(Clone, Copy)]
pub struct Motion<'a> {
    pub win_dy: &'a [i32],
    pub sel: Option<Rect>,
}

/// Nessun movimento: la scrivania ferma (GBA e test).
pub const STILL: Motion<'static> = Motion { win_dy: &[], sel: None };

pub fn draw<C: Canvas>(c: &mut C, s: &Shell) {
    draw_motion(c, s, &STILL);
}

/// Come `draw`, con finestre e selettore spostati da `m` (apertura, chiusura, scorrimento).
pub fn draw_motion<C: Canvas>(c: &mut C, s: &Shell, m: &Motion<'_>) {
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
    // Selettore in volo: sotto le icone, così il contorno resta leggibile durante lo scorrimento.
    if let (Some(r), true) = (m.sel, c.rich()) {
        c.fill(r.inset(-1), T1);
    }

    // Icone: quadrato T2, selezionata T1, aperta con sottolineatura T2.
    let (start, count) = s.visible();
    for k in 0..count {
        let i = start + k;
        let ic = &s.icons[i];
        let cell = icon_cell(k);
        if c.rich() {
            if i == s.sel && m.sel.is_none() {
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
        let dy = m.win_dy.get(i).copied().unwrap_or(0);
        let f = Rect::new(w.frame.x, w.frame.y + dy, w.frame.w, w.frame.h);
        shadow(c, f);
        c.rect_rounded(f, CORNER, T3);
        let tb = title_bar(f);
        let rich = c.rich();
        // x86: titolo dell'app nel suo colore (attiva) con icona 8×8 a sinistra; GBA: T2/T1 come da bibbia.
        let tx = if rich {
            // Striscia del titolo prolungata di un raggio: gli angoli bassi finiscono sotto il corpo.
            let te = Rect::new(tb.x, tb.y, tb.w, tb.h + CORNER);
            c.rect_rounded(te, CORNER, if Some(i) == active { Tok::A(w.app) } else { T1 });
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
        c.rect_rounded(close_button(f), CLOSE / 2, cl);
        c.rect_rounded(max_button(f), CLOSE / 2, mx);
        c.rect_rounded(min_button(f), CLOSE / 2, mn);
        let sc = if rich { T0 } else { T1 };
        glyph(c, close_button(f), 'x', sc);
        glyph(c, max_button(f), '+', sc);
        glyph(c, min_button(f), '-', sc);
        for r in [close_button(f), max_button(f), min_button(f)] {
            corners(c, r);
        }
        c.rect_rounded(body(f), CORNER, T1);
        draw_app(c, s, w.app, content(f));
        if c.rounded() && !c.smooth() {
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
