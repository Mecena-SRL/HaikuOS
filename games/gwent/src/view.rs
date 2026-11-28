//! Disegno (solo host, std). Tabellone in coordinate di progetto 1920×1080, scalato con la stessa
//! proporzione in ogni schermo (letterbox). Primitive vettoriali di `haiku-gui` più sfumature, bagliori
//! e velature calcolati per pixel (nessun bitmap). Testo solo ASCII.

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

// Palette della scena (0xRRGGBB).
const SKY_TOP: u32 = 0x08101F;
const SKY_BOT: u32 = 0x1B3354;
const FELT_TOP: u32 = 0x1F6B4F;
const FELT_BOT: u32 = 0x0B2C22;
const ROW_TOP: u32 = 0x0A2A20;
const ROW_BOT: u32 = 0x0E3A2C;
const GOLD_TOP: u32 = 0xFFE08A;
const GOLD_BOT: u32 = 0xC98E2A;
const PAPER_TOP: u32 = 0xFCF8F0;
const PAPER_BOT: u32 = 0xE6D6BC;
const PL: u32 = 0x4E9BFF;
const AI: u32 = 0xFF8A3D;
const FROST: u32 = 0x9FD8FF;
const FOG: u32 = 0xC4CEDD;
const GLOW_GOLD: u32 = 0xFFD76A;
const BTN_TOP: u32 = 0x47E6C4;
const BTN_BOT: u32 = 0x1A9C84;
const BTN_OFF: u32 = 0x4A5368;

/// Interpolazione lineare di due colori, `t` in 0..=255.
fn lerp(a: u32, b: u32, t: u32) -> u32 {
    let ch = |s: u32| {
        let (x, y) = ((a >> s) & 255, (b >> s) & 255);
        ((x * (255 - t) + y * t) / 255) << s
    };
    ch(16) | ch(8) | ch(0)
}

/// Mescola `src` su `dst` con copertura `a` (0..=255).
fn mix(dst: u32, src: u32, a: u32) -> u32 {
    let ch = |s: u32| {
        let (d, c) = ((dst >> s) & 255, (src >> s) & 255);
        ((d * (255 - a) + c * a) / 255) << s
    };
    ch(16) | ch(8) | ch(0)
}

/// Fotogramma RGB, `w`×`h` pixel, formato 0x00RRGGBB (lo stesso di minifb).
pub struct Img {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u32>,
}

impl Img {
    /// Rettangolo arrotondato con sfumatura verticale da `c0` (in alto) a `c1` (in basso). `rad` 0 = rettangolo.
    fn rr_grad(&mut self, r: Rect, rad: i32, c0: u32, c1: u32) {
        let rad = rad.min(r.w / 2).min(r.h / 2).max(0);
        let (x0, x1) = (r.x.max(0), (r.x + r.w).min(self.w as i32));
        let (y0, y1) = (r.y.max(0), (r.y + r.h).min(self.h as i32));
        for y in y0..y1 {
            let c = lerp(c0, c1, ((y - r.y) * 255 / (r.h - 1).max(1)) as u32);
            for x in x0..x1 {
                let (dx, dy) = (x - r.x, y - r.y);
                let ox = if dx < rad { rad - 1 - dx } else if dx >= r.w - rad { dx - (r.w - rad) } else { 0 };
                let oy = if dy < rad { rad - 1 - dy } else if dy >= r.h - rad { dy - (r.h - rad) } else { 0 };
                if ox * ox + oy * oy < rad * rad || (ox == 0 && oy == 0) {
                    self.px[y as usize * self.w + x as usize] = c;
                }
            }
        }
    }

    /// Velatura `c` di copertura `a` su tutto il rettangolo.
    fn tint(&mut self, r: Rect, a: u32, c: u32) {
        for y in r.y.max(0)..(r.y + r.h).min(self.h as i32) {
            for x in r.x.max(0)..(r.x + r.w).min(self.w as i32) {
                let i = y as usize * self.w + x as usize;
                self.px[i] = mix(self.px[i], c, a);
            }
        }
    }

    /// Alone attorno al rettangolo: `amax` al bordo, che svanisce su `spread` pixel con caduta quadratica.
    fn glow(&mut self, r: Rect, spread: i32, c: u32, amax: u32) {
        let s2 = spread * spread;
        for y in (r.y - spread).max(0)..(r.y + r.h + spread).min(self.h as i32) {
            for x in (r.x - spread).max(0)..(r.x + r.w + spread).min(self.w as i32) {
                let dx = (r.x - x).max(x - (r.x + r.w - 1)).max(0);
                let dy = (r.y - y).max(y - (r.y + r.h - 1)).max(0);
                let d2 = dx * dx + dy * dy;
                if d2 == 0 || d2 >= s2 {
                    continue;
                }
                let a = amax * (s2 - d2) as u32 / s2 as u32;
                let i = y as usize * self.w + x as usize;
                self.px[i] = mix(self.px[i], c, a);
            }
        }
    }

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
        self.rr_grad(r, 0, t.rgb(), t.rgb());
    }

    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok) {
        vfont::draw_text(self, x, y, 16, s, t);
    }

    fn rect_rounded(&mut self, r: Rect, radius: i32, t: Tok) {
        self.rr_grad(r, radius, t.rgb(), t.rgb());
    }

    fn blend(&mut self, x: i32, y: i32, a: u8, t: Tok) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        let i = y as usize * self.w + x as usize;
        self.px[i] = mix(self.px[i], t.rgb(), a as u32);
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

    fn s(&self, v: i32) -> i32 {
        (v as f32 * self.k) as i32
    }

    /// Rettangolo arrotondato con sfumatura da `c0` a `c1`.
    fn rr(&mut self, r: Rect, rad: i32, c0: u32, c1: u32) {
        let d = self.d(r);
        let rad = self.s(rad);
        self.im.rr_grad(d, rad, c0, c1);
    }

    /// Rettangolo arrotondato di tinta piatta.
    fn rr_flat(&mut self, r: Rect, rad: i32, c: u32) {
        self.rr(r, rad, c, c);
    }

    fn grad(&mut self, r: Rect, c0: u32, c1: u32) {
        let d = self.d(r);
        self.im.rr_grad(d, 0, c0, c1);
    }

    fn tint(&mut self, r: Rect, a: u32, c: u32) {
        let d = self.d(r);
        self.im.tint(d, a, c);
    }

    fn glow(&mut self, r: Rect, spread: i32, c: u32, amax: u32) {
        let d = self.d(r);
        let sp = self.s(spread);
        self.im.glow(d, sp, c, amax);
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

fn owner_color(owner: usize) -> u32 {
    if owner == 0 { PL } else { AI }
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

/// Una carta: cornice del colore del proprietario, carta a sfumatura con riflesso in alto,
/// potenza in medaglione dorato, nome e tipo. `played` accende l'alone del proprietario.
fn card(p: &mut Pane, r: Rect, id: usize, owner: usize, power: i32, sel: bool, played: bool) {
    if played {
        p.glow(r, 18, owner_color(owner), 150);
    }
    if sel {
        p.glow(r, 26, GLOW_GOLD, 230);
    }
    p.rr_flat(r, 10, owner_color(owner));
    let inner = Rect::new(r.x + 5, r.y + 5, r.w - 10, r.h - 10);
    p.rr(inner, 8, PAPER_TOP, PAPER_BOT);
    p.tint(Rect::new(inner.x + 8, inner.y + 3, inner.w - 16, inner.h / 3), 70, 0xFFFFFF);
    let d = DEFS[id];
    let weather = matches!(d.kind, Kind::Frost | Kind::Fog | Kind::Clear);
    if !weather {
        p.rr(Rect::new(r.x + 9, r.y + 9, 34, 34), 17, GOLD_TOP, GOLD_BOT);
        let s = format!("{power}");
        p.txt(r.x + 17, r.y + 13, 26, &s, Tok::T0);
    }
    p.txt(r.x + 12, r.y + r.h - 62, 18, d.name, Tok::T0);
    p.txt(r.x + 12, r.y + r.h - 32, 14, tag(id), Tok::T2);
}

/// Tabellone, mano e pannelli in `p` (riusato tra i fotogrammi). `cur` è la carta selezionata, `msg` la riga di stato.
pub fn frame_into(p: &mut Pane, g: &Game, cur: usize, msg: &str) {
    // Sfondo a sfumatura e tavolo di feltro con bordo d'oro.
    p.grad(Rect::new(0, 0, DW, DH), SKY_TOP, SKY_BOT);
    p.rr_flat(Rect::new(262, 84, 1296, 700), 28, GOLD_BOT);
    p.rr(Rect::new(268, 90, 1284, 688), 24, FELT_TOP, FELT_BOT);

    // Intestazione: avversario a sinistra, tu a destra, round al centro.
    p.txt(40, 30, 30, "AVVERSARIO", Tok::T1);
    p.txt(1560, 30, 30, "TU", Tok::T1);
    let round = format!("ROUND {}", g.round);
    p.txt_c(Rect::new(0, 0, DW, 0), 30, 30, &round, Tok::T1);
    for (b, x0) in [(1usize, 40), (0, 1560)] {
        for i in 0..2 {
            let r = Rect::new(x0 + 240 + i * 34, 34, 22, 22);
            let c = if (i as u8) < g.side[b].wins { owner_color(b) } else { 0x3A4660 };
            p.rr_flat(r, 11, c);
        }
    }

    // File: sfondo scuro, velatura del meteo, carte con alone.
    for board in 0..2 {
        for row in 0..2 {
            let rr = row_rect(board, row);
            p.rr(rr, 14, ROW_TOP, ROW_BOT);
            let affected = (row == 0 && g.frost) || (row == 1 && g.fog);
            if affected {
                let c = if row == 0 { FROST } else { FOG };
                p.tint(rr, 80, c);
            }
            let slots = g.side[board].rows[row];
            let n = slots.n as usize;
            let pitch = if n <= 1 { 130 } else { (1100 / (n as i32 - 1)).min(130) };
            for i in 0..n {
                let slot = slots.c[i];
                let owner = (slot >> 7) as usize;
                let id = (slot & 0x7f) as usize;
                let power = g.slot_power(board, row, i);
                let cr = Rect::new(rr.x + 4 + i as i32 * pitch, rr.y + 15, 120, 130);
                card(p, cr, id, owner, power, false, true);
            }
        }
    }

    // Punteggi per tabellone (sinistra).
    for (board, y) in [(1usize, 110), (0, 450)] {
        p.rr_flat(Rect::new(40, y, 220, 310), 14, GOLD_BOT);
        p.rr(Rect::new(43, y + 3, 214, 304), 12, 0x2A4A7A, 0x142744);
        let label = if board == 0 { "TU" } else { "AVV." };
        p.txt_c(Rect::new(40, 0, 220, 0), y + 20, 26, label, Tok::T1);
        let s = format!("{}", g.score(board));
        p.txt_c(Rect::new(40, 0, 220, 0), y + 120, 96, &s, Tok::T1);
    }

    // Mano: la carta selezionata si alza e viene disegnata per ultima, sopra le altre.
    let n = g.side[0].hand.n as usize;
    let mut order: Vec<usize> = (0..n).filter(|&i| i != cur).collect();
    if cur < n {
        order.push(cur);
    }
    for i in order {
        let id = g.side[0].hand.c[i] as usize;
        let mut r = hand_rect(i, n);
        let sel = i == cur;
        if sel {
            r.y -= 20;
        }
        let power = DEFS[id].power as i32;
        card(p, r, id, 0, power, sel, false);
    }

    // Pannello di destra: stato, meteo, pulsante.
    p.txt(1560, 130, 24, &format!("Mazzo tu: {}", g.side[0].deck.n), Tok::T1);
    p.txt(1560, 165, 24, &format!("Mano avv.: {}", g.side[1].hand.n), Tok::T1);
    p.txt(1560, 220, 24, if g.frost { "GELO attivo" } else { "" }, Tok::A(4));
    p.txt(1560, 255, 24, if g.fog { "NEBBIA attiva" } else { "" }, Tok::A(4));
    let yours = g.turn == 0 && g.over.is_none();
    let label = if yours { "PASSA" } else { "..." };
    if yours {
        p.rr(PASS, 12, BTN_TOP, BTN_BOT);
    } else {
        p.rr_flat(PASS, 12, BTN_OFF);
    }
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
