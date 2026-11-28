//! Disegno (solo host, std). Tabellone in coordinate di progetto 1920×1080, scalato con la stessa
//! proporzione in ogni schermo (letterbox). Primitive vettoriali di `haiku-gui` più sfumature, bagliori
//! e velature calcolati per pixel (nessun bitmap). Testo solo ASCII.

use gwent::cards::{Kind, DEFS};
use gwent::{Game, Round};
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

/// Rettangolo della carta `i` in mano (`n` carte in totale). La carta selezionata si alza di 20.
pub fn card_rect(i: usize, n: usize, sel: bool) -> Rect {
    let pitch = if n <= 1 { 0 } else { (1050 / (n as i32 - 1)).min(170) };
    Rect::new(300 + i as i32 * pitch, if sel { 810 } else { 830 }, 150, 220)
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

/// Cosa c'è sotto il punto `(x, y)` (coordinate di progetto). La carta `cur` è disegnata per ultima,
/// quindi è controllata per prima.
pub fn hit(x: i32, y: i32, hand_len: usize, cur: usize) -> Option<Hit> {
    if inside(PASS, x, y) {
        return Some(Hit::Pass);
    }
    if cur < hand_len && inside(card_rect(cur, hand_len, true), x, y) {
        return Some(Hit::Card(cur));
    }
    (0..hand_len).rev().find(|&i| i != cur && inside(card_rect(i, hand_len, false), x, y)).map(Hit::Card)
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
const PANEL_TOP: u32 = 0x16355E;
const PANEL_BOT: u32 = 0x0D1F3D;
const PL: u32 = 0x4E9BFF;
const AI: u32 = 0xFF8A3D;
const FROST: u32 = 0x9FD8FF;
const FOG: u32 = 0xC4CEDD;
const GLOW_GOLD: u32 = 0xFFD76A;
const BTN_TOP: u32 = 0x47E6C4;
const BTN_BOT: u32 = 0x1A9C84;
const BTN_WARN: u32 = 0xF08A3C;
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

/// Spezza `s` in righe di al più `max` caratteri, a parole intere.
fn wrap(s: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for w in s.split_whitespace() {
        if !cur.is_empty() && cur.len() + 1 + w.len() > max {
            out.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(w);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Fotogramma `w`×`h` pixel, formato 0x00RRGGBB (lo stesso di minifb).
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

    /// Testo con angolo in alto a sinistra; `size` in unità di progetto. Minimo 9 px sullo schermo.
    fn txt(&mut self, x: i32, y: i32, size: i32, s: &str, t: Tok) {
        let px = self.ox + (x as f32 * self.k) as i32;
        let py = self.oy + (y as f32 * self.k) as i32;
        let sz = ((size as f32 * self.k) as i32).max(9);
        vfont::draw_text(&mut self.im, px, py, sz, s, t);
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

/// Etichetta breve sotto il nome della carta.
fn tag(id: usize) -> &'static str {
    let d = DEFS[id];
    match d.kind {
        Kind::Unit => match d.group {
            1 => "LUPI",
            2 => "ARCIERI",
            _ => "",
        },
        Kind::Morale => "MORALE",
        Kind::Medic => "GUARIGIONE",
        Kind::Spy => "SPIA",
        Kind::Horn => "CORNO",
        Kind::Scorch => "SCORCH",
        Kind::Frost | Kind::Fog | Kind::Clear => "METEO",
    }
}

/// Una carta: cornice del colore del proprietario (oro se selezionata), carta a sfumatura con riflesso,
/// potenza in medaglione dorato. Se la potenza differisce da quella base, accanto compare "/base".
/// `played` accende l'alone del proprietario sull'ultima carta piazzata.
fn card(p: &mut Pane, r: Rect, id: usize, owner: usize, power: i32, sel: bool, played: bool) {
    let d = DEFS[id];
    let col = owner_color(owner);
    if played {
        p.glow(r, 14, col, 140);
    }
    if sel {
        p.glow(r, 14, GLOW_GOLD, 120);
    }
    p.rr_flat(r, 10, if sel { GLOW_GOLD } else { col });
    let inner = Rect::new(r.x + 4, r.y + 4, r.w - 8, r.h - 8);
    p.rr(inner, 8, PAPER_TOP, PAPER_BOT);
    p.tint(Rect::new(inner.x + 8, inner.y + 3, inner.w - 16, inner.h / 3), 60, 0xFFFFFF);
    let weather = matches!(d.kind, Kind::Frost | Kind::Fog | Kind::Clear | Kind::Scorch);
    if !weather {
        p.rr(Rect::new(r.x + 8, r.y + 8, 36, 36), 18, GOLD_TOP, GOLD_BOT);
        let s = if d.kind == Kind::Horn { "x2".to_string() } else { format!("{power}") };
        p.txt(r.x + 14, r.y + 14, 26, &s, Tok::T0);
        if d.kind == Kind::Unit && power != d.power as i32 {
            p.txt(r.x + 48, r.y + 16, 16, &format!("/{}", d.power), Tok::T0);
        }
    }
    p.txt(r.x + 10, r.y + r.h - 64, 18, d.name, Tok::T0);
    p.txt(r.x + 10, r.y + r.h - 34, 15, tag(id), Tok::T0);
}

/// Tabellone, mano e pannelli in `p` (riusato tra i fotogrammi). `cur` è la carta selezionata, `msg` la riga di stato.
pub fn frame_into(p: &mut Pane, g: &Game, cur: usize, msg: &str) {
    // Sfondo a sfumatura e tavolo di feltro con bordo d'oro.
    p.grad(Rect::new(0, 0, DW, DH), SKY_TOP, SKY_BOT);
    p.rr_flat(Rect::new(262, 84, 1296, 700), 28, GOLD_BOT);
    p.rr(Rect::new(268, 90, 1284, 688), 24, FELT_TOP, FELT_BOT);

    // Intestazioni: nomi, round vinti (pallini con etichetta), numero del round e regola.
    p.txt(40, 16, 28, "AVVERSARIO", Tok::T1);
    p.txt(1560, 16, 28, "TU", Tok::T1);
    p.txt_c(Rect::new(0, 0, DW, 0), 12, 40, &format!("ROUND {}", g.round), Tok::T1);
    p.txt_c(Rect::new(0, 0, DW, 0), 62, 17, "Vince chi ha piu potenza sul tabellone. Primo a 2 round.", Tok::T1);
    for (b, x0) in [(1usize, 40), (0usize, 1560)] {
        for i in 0..2i32 {
            let c = if (i as u8) < g.side[b].wins { owner_color(b) } else { 0x3A4660 };
            p.rr_flat(Rect::new(x0 + i * 34, 58, 24, 24), 12, c);
        }
        p.txt(if b == 1 { 112 } else { 1640 }, 62, 16, "ROUND VINTI", Tok::T1);
    }

    // File: sfondo scuro, velatura e scritta del meteo, carte con alone sull'ultima giocata.
    for board in 0..2 {
        for row in 0..2 {
            let rr = row_rect(board, row);
            p.rr(rr, 14, ROW_TOP, ROW_BOT);
            let affected = (row == 0 && g.frost) || (row == 1 && g.fog);
            if affected {
                let c = if row == 0 { FROST } else { FOG };
                p.tint(rr, 80, c);
                let note = if row == 0 { "GELO: potenza 1" } else { "NEBBIA: potenza 1" };
                p.txt(rr.x + rr.w - 220, rr.y + 1, 16, note, Tok::A(4));
            }
            p.txt(rr.x + 6, rr.y + 1, 15, if row == 0 { "VICINA" } else { "A DISTANZA" }, Tok::T1);
            let slots = g.side[board].rows[row];
            let n = slots.n as usize;
            let pitch = if n <= 1 { 130 } else { (1100 / (n as i32 - 1)).min(130) };
            for i in 0..n {
                let slot = slots.c[i];
                let owner = (slot >> 7) as usize;
                let id = (slot & 0x7f) as usize;
                let power = g.slot_power(board, row, i);
                let played = g.last == Some((board as u8, row as u8, slot));
                let cr = Rect::new(rr.x + 4 + i as i32 * pitch, rr.y + 15, 120, 130);
                card(p, cr, id, owner, power, false, played);
            }
        }
    }

    // Punteggi per tabellone (sinistra). Il tabellone di turno ha l'alone del suo colore.
    for (board, y) in [(1usize, 110), (0, 450)] {
        let box_r = Rect::new(40, y, 220, 310);
        if g.over.is_none() && g.turn as usize == board {
            p.glow(box_r, 16, owner_color(board), 150);
        }
        p.rr_flat(box_r, 14, GOLD_BOT);
        p.rr(Rect::new(43, y + 3, 214, 304), 12, PANEL_TOP, PANEL_BOT);
        let label = if board == 0 { "TU" } else { "AVV." };
        p.txt_c(Rect::new(40, 0, 220, 0), y + 20, 26, label, Tok::T1);
        let s = format!("{}", g.score(board));
        p.txt_c(Rect::new(40, 0, 220, 0), y + 120, 96, &s, Tok::T1);
        p.txt_c(Rect::new(40, 0, 220, 0), y + 255, 18, "POTENZA", Tok::T1);
    }

    // Mano: la carta selezionata si alza e viene disegnata per ultima, sopra le altre.
    let n = g.side[0].hand.n as usize;
    let mut order: Vec<usize> = (0..n).filter(|&i| i != cur).collect();
    if cur < n {
        order.push(cur);
    }
    for i in order {
        let id = g.side[0].hand.c[i] as usize;
        let sel = i == cur;
        let r = card_rect(i, n, sel);
        card(p, r, id, 0, DEFS[id].power as i32, sel, false);
    }

    // Pannello di destra: conteggi, meteo, descrizione della carta selezionata.
    p.txt(1560, 108, 22, &format!("Mano tu: {}   Mazzo: {}", g.side[0].hand.n, g.side[0].deck.n), Tok::T1);
    p.txt(1560, 138, 22, &format!("Mano avv.: {}", g.side[1].hand.n), Tok::T1);
    let meteo = match (g.frost, g.fog) {
        (true, true) => "Meteo: GELO e NEBBIA",
        (true, false) => "Meteo: GELO",
        (false, true) => "Meteo: NEBBIA",
        _ => "Meteo: nessuno",
    };
    p.txt(1560, 170, 22, meteo, Tok::A(4));

    let panel = Rect::new(1556, 206, 324, 226);
    p.rr_flat(panel, 12, GOLD_BOT);
    p.rr(Rect::new(panel.x + 3, panel.y + 3, panel.w - 6, panel.h - 6), 10, PANEL_TOP, PANEL_BOT);
    if cur < n {
        let d = DEFS[g.side[0].hand.c[cur] as usize];
        p.txt(1572, 218, 26, d.name, Tok::T3);
        p.txt(1572, 252, 20, &format!("Potenza {}", d.power), Tok::T1);
        for (k, line) in wrap(d.desc, 27).iter().enumerate() {
            p.txt(1572, 284 + k as i32 * 26, 20, line, Tok::T1);
        }
    } else {
        p.txt(1572, 218, 22, "Nessuna carta in mano", Tok::T1);
    }

    // Pulsante PASSA: arancio se stai perdendo il round, grigio se tocca all'avversario.
    let yours = g.turn == 0 && g.over.is_none();
    let behind = g.score(0) < g.score(1);
    if yours {
        let (top, bot, label) = if behind { (BTN_WARN, BTN_WARN, "PASSA: PERDI") } else { (BTN_TOP, BTN_BOT, "PASSA") };
        p.rr(PASS, 12, top, bot);
        p.txt_c(PASS, PASS.y + 22, 34, label, Tok::T0);
        if behind {
            p.txt(1560, 560, 18, "Passando perdi il round. Premi P due volte.", Tok::A(1));
        }
    } else {
        p.rr_flat(PASS, 12, BTN_OFF);
        p.txt_c(PASS, PASS.y + 22, 34, "ATTENDI", Tok::T1);
    }
    p.txt(1560, 596, 26, if yours { "Tocca a te" } else { "Turno avversario" }, Tok::T3);
    for (k, line) in wrap(msg, 30).iter().take(3).enumerate() {
        p.txt(1560, 634 + k as i32 * 24, 19, line, Tok::T1);
    }

    // Ultimo round concluso.
    if let Some(Round { pts, winner }) = g.result {
        let who = if winner == 0 { "Vince: tu" } else { "Vince: avversario" };
        p.txt(1560, 790, 20, "Ultimo round", Tok::T1);
        p.txt(1560, 816, 26, &format!("Tu {}  -  Avv {}", pts[0], pts[1]), Tok::T1);
        p.txt(1560, 850, 22, who, if winner == 0 { Tok::A(4) } else { Tok::A(3) });
    }

    // Fine partita: velo scuro e riepilogo.
    if let Some(w) = g.over {
        p.tint(Rect::new(0, 0, DW, DH), 150, 0x000000);
        let panel = Rect::new(560, 300, 800, 420);
        p.rr_flat(panel, 24, GOLD_BOT);
        p.rr(Rect::new(566, 306, 788, 408), 20, PANEL_TOP, PANEL_BOT);
        let title = if w == 0 { "HAI VINTO LA PARTITA" } else { "HAI PERSO LA PARTITA" };
        p.txt_c(panel, 340, 56, title, Tok::T3);
        p.txt_c(panel, 450, 36, &format!("Round vinti: tu {} - avv {}", g.side[0].wins, g.side[1].wins), Tok::T1);
        p.txt_c(panel, 600, 24, "Invio: nuova partita     Esc: esci", Tok::T1);
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
            assert_eq!(hit(dx, dy, 3, 0), Some(Hit::Pass));
        }
    }

    #[test]
    fn carta_in_mano_e_selezionabile() {
        let r = card_rect(2, 5, false);
        assert_eq!(hit(r.x + 5, r.y + 5, 5, 0), Some(Hit::Card(2)));
        assert_eq!(hit(5, 5, 5, 0), None);
    }

    #[test]
    fn carta_selezionata_ha_la_precedenza_dove_si_sovrappone() {
        // Con 10 carte la selezionata 3 copre parte della 2: un click sulla zona comune prende la 3.
        let sel = card_rect(3, 10, true);
        assert_eq!(hit(sel.x + 2, sel.y + 40, 10, 3), Some(Hit::Card(3)));
    }
}
