//! Disegno (solo host, std). Il tabellone è in coordinate di progetto 1920×1080, scalato con la stessa
//! proporzione in ogni schermo (letterbox). Usa il trait `Canvas` di `haiku-gui`. Testo solo ASCII.

use gwent::cards::{Kind, DEFS};
use gwent::Game;
use haiku_gui::layout::Rect;
use haiku_gui::palette::Tok;
use haiku_gui::render::Canvas;
use haiku_gui::vfont;

pub const DW: i32 = 1920;
pub const DH: i32 = 1080;

/// Pulsante PASSA.
pub const PASS: Rect = Rect::new(1560, 470, 300, 80);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hit {
    Card(usize),
    Pass,
}

/// Rettangolo della carta `i` in mano (`n` carte in totale).
pub fn hand_rect(i: usize, n: usize) -> Rect {
    let pitch = if n <= 1 { 0 } else { (1050 / (n as i32 - 1)).min(170) };
    Rect::new(300 + i as i32 * pitch, 830, 150, 220)
}

/// Riga `row` (0 vicina, 1 a distanza) del tabellone `board` (0 tu, 1 avversario).
fn row_rect(board: usize, row: usize) -> Rect {
    let y = match (board, row) {
        (0, 0) => 450,
        (0, _) => 610,
        (_, 0) => 270,
        _ => 110,
    };
    Rect::new(300, y, 1220, 150)
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
}

/// Cosa c'è sotto il punto `(x, y)` (coordinate di progetto).
pub fn hit(x: i32, y: i32, hand_len: usize) -> Option<Hit> {
    if inside(PASS, x, y) {
        return Some(Hit::Pass);
    }
    (0..hand_len).rev().find(|&i| inside(hand_rect(i, hand_len), x, y)).map(Hit::Card)
}

/// Fotogramma `w`×`h` pixel, formato 0x00RRGGBB (lo stesso di minifb).
pub struct Img {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u32>,
}

fn rgb(t: Tok) -> u32 {
    t.rgb()
}

impl Img {
    /// PPM P6 a dimensioni native.
    pub fn write_ppm(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        write!(out, "P6\n{} {}\n255\n", self.w, self.h)?;
        for &p in &self.px {
            out.write_all(&[(p >> 16) as u8, (p >> 8) as u8, p as u8])?;
        }
        Ok(())
    }
}

impl Canvas for Img {
    fn fill(&mut self, r: Rect, t: Tok) {
        let c = rgb(t);
        for y in r.y.max(0)..(r.y + r.h).min(self.h as i32) {
            for x in r.x.max(0)..(r.x + r.w).min(self.w as i32) {
                self.px[y as usize * self.w + x as usize] = c;
            }
        }
    }

    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok) {
        vfont::draw_text(self, x, y, 16, s, t);
    }

    fn rect_rounded(&mut self, r: Rect, radius: i32, t: Tok) {
        let c = rgb(t);
        let rad = radius.min(r.w / 2).min(r.h / 2).max(0);
        for y in r.y.max(0)..(r.y + r.h).min(self.h as i32) {
            for x in r.x.max(0)..(r.x + r.w).min(self.w as i32) {
                let (dx, dy) = (x - r.x, y - r.y);
                let ox = if dx < rad { rad - 1 - dx } else if dx >= r.w - rad { dx - (r.w - rad) } else { 0 };
                let oy = if dy < rad { rad - 1 - dy } else if dy >= r.h - rad { dy - (r.h - rad) } else { 0 };
                if ox * ox + oy * oy < rad * rad || (ox == 0 && oy == 0) {
                    self.px[y as usize * self.w + x as usize] = c;
                }
            }
        }
    }

    fn blend(&mut self, x: i32, y: i32, a: u8, t: Tok) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        let d = rgb(t);
        let i = y as usize * self.w + x as usize;
        let s = self.px[i];
        let mix = |sh: u32| {
            let (s0, d0, a) = ((s >> sh) & 255, (d >> sh) & 255, a as u32);
            ((s0 * (255 - a) + d0 * a) / 255) << sh
        };
        self.px[i] = mix(16) | mix(8) | mix(0);
    }
}

/// Converte le coordinate di progetto in pixel dello schermo di uscita.
pub struct Pane {
    pub im: Img,
    k: f32,
    ox: i32,
    oy: i32,
}

impl Pane {
    pub fn new(w: usize, h: usize) -> Self {
        let k = (w as f32 / DW as f32).min(h as f32 / DH as f32);
        let ox = ((w as f32 - DW as f32 * k) / 2.0) as i32;
        let oy = ((h as f32 - DH as f32 * k) / 2.0) as i32;
        Pane { im: Img { w, h, px: vec![0; w * h] }, k, ox, oy }
    }

    /// Pixel dello schermo → coordinate di progetto (per il mouse).
    pub fn to_design(&self, px: i32, py: i32) -> (i32, i32) {
        (((px - self.ox) as f32 / self.k) as i32, ((py - self.oy) as f32 / self.k) as i32)
    }

    fn d(&self, r: Rect) -> Rect {
        let s = |v: i32| (v as f32 * self.k) as i32;
        Rect::new(self.ox + s(r.x), self.oy + s(r.y), s(r.w), s(r.h))
    }

    fn fill(&mut self, r: Rect, t: Tok) {
        let d = self.d(r);
        self.im.fill(d, t);
    }

    fn rr(&mut self, r: Rect, rad: i32, t: Tok) {
        let d = self.d(r);
        self.im.rect_rounded(d, (rad as f32 * self.k) as i32, t);
    }

    /// Testo con angolo in alto a sinistra; `size` in unità di progetto (em).
    fn txt(&mut self, x: i32, y: i32, size: i32, s: &str, t: Tok) {
        let px = self.ox + (x as f32 * self.k) as i32;
        let py = self.oy + (y as f32 * self.k) as i32;
        vfont::draw_text(&mut self.im, px, py, (size as f32 * self.k) as i32, s, t);
    }

    /// Testo centrato orizzontalmente in `r` (larghezza stimata: metà em per carattere).
    fn txt_c(&mut self, r: Rect, y: i32, size: i32, s: &str, t: Tok) {
        let w = s.len() as i32 * size / 2;
        self.txt(r.x + (r.w - w) / 2, y, size, s, t);
    }
}

fn accent(owner: usize) -> Tok {
    if owner == 0 { Tok::A(4) } else { Tok::A(3) }
}

fn tag(id: usize) -> &'static str {
    let d = DEFS[id];
    match d.kind {
        Kind::Unit if d.group != 0 => "GRUPPO",
        Kind::Unit => "",
        Kind::Morale => "MORALE",
        Kind::Medic => "MEDICO",
        Kind::Spy => "SPIA",
        Kind::Frost => "METEO GELO",
        Kind::Fog => "METEO NEBBIA",
        Kind::Clear => "METEO SERENO",
    }
}

/// Una carta: cornice del colore del proprietario, potenza in alto a sinistra, nome e tipo in basso.
fn card(p: &mut Pane, r: Rect, id: usize, owner: usize, power: i32, sel: bool) {
    if sel {
        p.rr(Rect::new(r.x - 5, r.y - 5, r.w + 10, r.h + 10), 12, Tok::T3);
    }
    p.rr(r, 10, accent(owner));
    p.rr(Rect::new(r.x + 5, r.y + 5, r.w - 10, r.h - 10), 8, Tok::T1);
    let d = DEFS[id];
    let weather = matches!(d.kind, Kind::Frost | Kind::Fog | Kind::Clear);
    if !weather {
        p.rr(Rect::new(r.x + 9, r.y + 9, 34, 34), 17, Tok::A(1));
        let s = format!("{power}");
        p.txt(r.x + 17, r.y + 13, 26, &s, Tok::T0);
    }
    p.txt(r.x + 12, r.y + r.h - 62, 18, d.name, Tok::T0);
    p.txt(r.x + 12, r.y + r.h - 32, 14, tag(id), Tok::T2);
}

/// Tabellone e pannelli in `p` (riusato tra i fotogrammi). `cur` è la carta selezionata, `msg` la riga di stato.
pub fn frame_into(p: &mut Pane, g: &Game, cur: usize, msg: &str) {
    p.im.px.fill(rgb(Tok::T0));
    p.fill(Rect::new(0, 0, DW, DH), Tok::T0);

    // Intestazione: avversario a sinistra, tu a destra, round al centro.
    p.txt(40, 30, 30, "AVVERSARIO", Tok::T1);
    p.txt(1560, 30, 30, "TU", Tok::T1);
    let round = format!("ROUND {}", g.round);
    p.txt_c(Rect::new(0, 0, DW, 0), 30, 30, &round, Tok::T1);
    for (b, x0) in [(1usize, 40), (0, 1560)] {
        for i in 0..2 {
            let r = Rect::new(x0 + 240 + i * 34, 34, 22, 22);
            p.rr(r, 11, if (i as u8) < g.side[b].wins { Tok::A(3) } else { Tok::T2 });
        }
    }

    // Tabellone: per ogni fila colore di sfondo e carte; la fila colpita dal meteo cambia colore.
    for board in 0..2 {
        for row in 0..2 {
            let affected = (row == 0 && g.frost) || (row == 1 && g.fog);
            let rr = row_rect(board, row);
            p.rr(rr, 14, if affected { Tok::A(4) } else { Tok::T2 });
            let slots = g.side[board].rows[row];
            let n = slots.n as usize;
            let pitch = if n <= 1 { 130 } else { (1100 / (n as i32 - 1)).min(130) };
            for i in 0..n {
                let slot = slots.c[i];
                let owner = (slot >> 7) as usize;
                let id = (slot & 0x7f) as usize;
                let power = g.slot_power(board, row, i);
                let cr = Rect::new(rr.x + 4 + i as i32 * pitch, rr.y + 15, 120, 130);
                card(p, cr, id, owner, power, false);
            }
        }
    }

    // Punteggi per tabellone (sinistra).
    for (board, y) in [(1usize, 110), (0, 450)] {
        p.rr(Rect::new(40, y, 220, 310), 14, Tok::T2);
        let label = if board == 0 { "TU" } else { "AVV." };
        p.txt_c(Rect::new(40, 0, 220, 0), y + 20, 26, label, Tok::T0);
        let s = format!("{}", g.score(board));
        p.txt_c(Rect::new(40, 0, 220, 0), y + 120, 96, &s, Tok::T0);
    }

    // Mano: la carta selezionata si alza.
    let n = g.side[0].hand.n as usize;
    for i in 0..n {
        let id = g.side[0].hand.c[i] as usize;
        let mut r = hand_rect(i, n);
        let sel = i == cur;
        if sel {
            r.y -= 20;
        }
        let power = DEFS[id].power as i32;
        card(p, r, id, 0, power, sel);
    }

    // Pannello di destra: stato, meteo, pulsante.
    p.txt(1560, 130, 24, &format!("Mazzo tu: {}", g.side[0].deck.n), Tok::T1);
    p.txt(1560, 165, 24, &format!("Mano avv.: {}", g.side[1].hand.n), Tok::T1);
    p.txt(1560, 220, 24, if g.frost { "GELO attivo" } else { "" }, Tok::A(4));
    p.txt(1560, 255, 24, if g.fog { "NEBBIA attiva" } else { "" }, Tok::A(4));
    let yours = g.turn == 0 && g.over.is_none();
    let (bg, label) = if yours { (Tok::A(2), "PASSA") } else { (Tok::T2, "...") };
    p.rr(PASS, 12, bg);
    p.txt_c(PASS, PASS.y + 22, 36, label, Tok::T0);
    p.txt(1560, 580, 26, if yours { "Tocca a te" } else { "Turno avversario" }, Tok::T3);
    p.txt(1560, 630, 24, msg, Tok::T1);
    if let Some(w) = g.over {
        let t = if w == 0 { "HAI VINTO" } else { "HAI PERSO" };
        p.txt_c(Rect::new(0, 0, DW, 0), 440, 64, t, Tok::T3);
    }
}

/// Fotogramma nuovo di `w`×`h` (per le immagini PPM).
pub fn frame(g: &Game, cur: usize, msg: &str, w: usize, h: usize) -> Pane {
    let mut p = Pane::new(w, h);
    frame_into(&mut p, g, cur, msg);
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_torna_in_coordinate_di_progetto() {
        for (w, h) in [(1280usize, 720usize), (800, 480), (1920, 1080)] {
            let p = Pane::new(w, h);
            let (cx, cy) = (1560 + 150, 510); // centro del pulsante PASSA
            let k = (w as f32 / DW as f32).min(h as f32 / DH as f32);
            let px = (p.ox as f32 + cx as f32 * k) as i32;
            let py = (p.oy as f32 + cy as f32 * k) as i32;
            let (dx, dy) = p.to_design(px, py);
            assert!((dx - cx).abs() <= 2 && (dy - cy).abs() <= 2, "{w}x{h}: {dx},{dy}");
            assert_eq!(hit(dx, dy, 3), Some(Hit::Pass));
        }
    }

    #[test]
    fn carta_in_mano_e_selezionabile() {
        let r = hand_rect(2, 5);
        assert_eq!(hit(r.x + 5, r.y + 5, 5), Some(Hit::Card(2)));
        assert_eq!(hit(5, 5, 5), None);
    }
}
