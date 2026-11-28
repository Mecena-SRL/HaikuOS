//! Disegno del gioco su `Canvas`, indipendente dal formato pixel. Tutto vettoriale
//! (rettangoli e ellissi), nessuna immagine. `no_std`.

use crate::battle::{Creature, Kind, Msg, Phase, MOVES, SPECIES};
use crate::world::{self, NpcKind, NPCS};
use crate::{Game, Mode};
use haiku_editor::gui::{Canvas, Pixel};

const INK: u32 = 0xFF1A202C;
const WHITE: u32 = 0xFFFFFFFF;
const PANEL: u32 = 0xFFF7FAFC;
const GREY: u32 = 0xFF4A5568;
const CURSOR: u32 = 0xFFECC94B;
const SKIN: u32 = 0xFFFED7AA;
const SHADOW: u32 = 0xFF276749;

/// Interpola due colori ARGB, `t` da 0 a 255.
fn mix(a: u32, b: u32, t: u32) -> u32 {
    let ch = |s: u32| {
        let (x, y) = ((a >> s) & 255, (b >> s) & 255);
        (x * (255 - t) + y * t) / 255
    };
    0xFF000000 | (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// Sfondo a gradiente verticale, a bande.
fn gradient<P: Pixel>(c: &mut Canvas<P>, x: usize, y: usize, w: usize, h: usize, top: u32, bot: u32) {
    const BANDS: usize = 48;
    for i in 0..BANDS {
        let (y0, y1) = (y + h * i / BANDS, y + h * (i + 1) / BANDS);
        c.rect(x, y0, w, y1 - y0, mix(top, bot, (i * 255 / (BANDS - 1)) as u32));
    }
}

fn disc<P: Pixel>(c: &mut Canvas<P>, cx: usize, cy: usize, r: usize, col: u32) {
    ellipse(c, cx, cy, r, r, col);
}

/// Ellisse piena, `cx >= rx` e `cy >= ry` richiesti.
fn ellipse<P: Pixel>(c: &mut Canvas<P>, cx: usize, cy: usize, rx: usize, ry: usize, col: u32) {
    let (rxi, ryi) = (rx as isize, ry as isize);
    for dy in -ryi..=ryi {
        let mut hw = rxi;
        while hw > 0 && hw * hw * ryi * ryi + dy * dy * rxi * rxi > rxi * rxi * ryi * ryi {
            hw -= 1;
        }
        c.rect(cx - hw as usize, (cy as isize + dy) as usize, (2 * hw + 1) as usize, 1, col);
    }
}

/// Pseudo-casualità stabile per tile (decora erba e pietre sempre allo stesso modo).
fn hash(x: usize, y: usize) -> usize {
    (x * 7919 + y * 104729 + x * y * 31) % 97
}

fn grass(y: usize) -> u32 {
    mix(0xFFC6F6D5, 0xFF68D391, (y * 255 / 11) as u32)
}

fn tall(y: usize) -> u32 {
    mix(0xFF48BB78, 0xFF276749, (y * 255 / 11) as u32)
}

/// Disegna l'intero schermo. La scala del testo dipende dall'altezza.
pub fn draw<P: Pixel>(c: &mut Canvas<P>, g: &Game) {
    let h = c.h;
    let s = (h / 150).max(2);
    let bh = h / 4;
    if g.mode == Mode::Fight {
        battle_view(c, g, s, bh);
    } else {
        overworld(c, g, s, bh);
    }
    if g.fade > 0 {
        wipe(c, g.fade);
    }
}

fn overworld<P: Pixel>(c: &mut Canvas<P>, g: &Game, s: usize, bh: usize) {
    let (w, h) = (c.w, c.h);
    let mh = h - bh;
    let town = g.map == 0;
    let (top, bot) = if town { (0xFF2B4C7E, 0xFFF6AD55) } else { (0xFF4A90C2, 0xFFBEE3F8) };
    gradient(c, 0, 0, w, mh, top, bot);
    let ts = (w / 16).min(mh / 12);
    let (ox, oy) = ((w - 16 * ts) / 2, (mh - 12 * ts) / 2);
    margins(c, town, ox, ox + 16 * ts, mh, ts);
    map_view(c, g, ox, oy, ts);
    let mut status = Msg::new();
    let (body, more) = if g.mode == Mode::Talk {
        let pages = g.text.split('|').count();
        (g.text.split('|').nth(g.page).unwrap_or(""), g.page + 1 < pages)
    } else {
        status.push(if town { "VERDEPOLI" } else { "PERCORSO 1" });
        status.push("   PALLE ");
        status.num(g.balls as i32);
        (status.as_str(), false)
    };
    dialog_box(c, 0, mh, w, bh, s, body, more);
}

/// Sfondo ai lati della mappa: skyline di case in città, colline sul percorso.
fn margins<P: Pixel>(c: &mut Canvas<P>, town: bool, left_end: usize, right_start: usize, mh: usize, ts: usize) {
    let bw = ts.max(4);
    let w = c.w;
    let mut x = 0;
    let mut i = 0;
    while x < w {
        let inside = x >= left_end && x < right_start;
        if !inside {
            let hgt = mh / 6 + hash(i, 3) * mh / 400;
            if town {
                c.rect(x, mh - hgt, bw - 2, hgt, 0xFF1F2A44);
                if hash(i, 5) % 2 == 0 {
                    c.rect(x + bw / 4, mh - hgt + bw / 3, (bw / 4).max(1), (bw / 4).max(1), 0xFFF6E05E);
                }
            } else {
                let r = (bw * 3 / 4).max(2);
                disc(c, x + r, mh, r, 0xFF22543D);
            }
        }
        x += bw;
        i += 1;
    }
}

fn map_view<P: Pixel>(c: &mut Canvas<P>, g: &Game, ox: usize, oy: usize, ts: usize) {
    let rows = world::map(g.map);
    for y in 0..12 {
        let row = rows[y].as_bytes();
        for x in 0..16 {
            tile(c, row[x] as char, x, y, ox + x * ts, oy + y * ts, ts);
        }
    }
    for n in NPCS.iter().filter(|n| n.map == g.map) {
        let col = match n.kind {
            NpcKind::Nurse => WHITE,
            NpcKind::Trainer if g.beaten => 0xFF4A4A5A,
            NpcKind::Trainer => 0xFF805AD5,
            NpcKind::Talk => CURSOR,
        };
        actor(c, ox + n.x as usize * ts, oy + n.y as usize * ts, ts, col);
    }
    actor(c, ox + g.x as usize * ts, oy + g.y as usize * ts, ts, 0xFFE53E3E);
}

fn tile<P: Pixel>(c: &mut Canvas<P>, ch: char, x: usize, y: usize, px: usize, py: usize, ts: usize) {
    let q = (ts / 4).max(1);
    match ch {
        ',' => {
            c.rect(px, py, ts, ts, tall(y));
            for k in 0..3 {
                let bx = px + ts * (2 * k + 1) / 7;
                let hgt = ts * (4 + hash(x, y + k) % 3) / 8;
                c.rect(bx, py + ts - hgt, (ts / 10).max(2), hgt, SHADOW);
                c.rect(bx, py + ts - hgt, (ts / 10).max(2), (ts / 12).max(1), 0xFF9AE6B4);
            }
        }
        '=' => {
            c.rect(px, py, ts, ts, 0xFFE2C78F);
            c.rect(px, py + ts - (ts / 10).max(1), ts, (ts / 10).max(1), 0xFFC9A96E);
            if hash(x, y) % 4 == 0 {
                c.rect(px + ts / 3, py + ts / 3, q, q, 0xFFC9A96E);
            }
        }
        'T' => {
            c.rect(px, py, ts, ts, grass(y));
            c.rect(px + ts * 2 / 5, py + ts * 3 / 5, ts / 5, ts * 2 / 5, 0xFF7B4F2B);
            let (cx, cy, r) = (px + ts / 2, py + ts * 2 / 5, ts * 2 / 5);
            disc(c, cx, cy, r, 0xFF22543D);
            disc(c, cx - ts / 8, cy - ts / 8, (ts / 6).max(1), 0xFF2F855A);
        }
        'H' => {
            c.rect(px, py, ts, ts, 0xFFE9D8A6);
            c.rect(px, py, ts, ts * 2 / 5, 0xFF9C4221);
            c.rect(px, py + ts * 2 / 5, ts, (ts / 12).max(1), 0xFF7B341E);
            c.rect(px + ts / 4, py + ts * 3 / 5, ts / 4, ts / 4, 0xFF63B3ED);
        }
        '#' => {
            c.rect(px, py, ts, ts, 0xFF2D3748);
            c.rect(px + q, py + q, ts - 2 * q, ts - 2 * q, GREY);
        }
        '<' | '>' => {
            c.rect(px, py, ts, ts, 0xFFE2C78F);
            c.rect(px + ts / 3, py + ts / 3, ts / 3, ts / 3, 0xFF975A16);
        }
        _ => {
            c.rect(px, py, ts, ts, grass(y));
            if hash(x, y) % 5 == 0 {
                c.rect(px + ts / 3, py + ts / 2, (ts / 12).max(1), ts / 4, SHADOW);
                c.rect(px + ts / 2, py + ts * 2 / 5, (ts / 12).max(1), ts / 4, SHADOW);
            }
        }
    }
}

/// Personaggio su tile: ombra, corpo a ellisse, occhi.
fn actor<P: Pixel>(c: &mut Canvas<P>, px: usize, py: usize, ts: usize, col: u32) {
    let (cx, cy) = (px + ts / 2, py + ts / 2);
    ellipse(c, cx, py + ts * 9 / 10, ts * 3 / 10, (ts / 12).max(1), SHADOW);
    ellipse(c, cx, cy, ts * 3 / 10, ts * 3 / 10, col);
    c.rect(cx - ts / 5, cy - ts / 8, (ts / 8).max(1), (ts / 8).max(1), INK);
    c.rect(cx + ts / 8, cy - ts / 8, (ts / 8).max(1), (ts / 8).max(1), INK);
    c.rect(cx - ts / 6, cy + ts / 6, ts / 3, (ts / 12).max(1), SKIN);
}

fn battle_view<P: Pixel>(c: &mut Canvas<P>, g: &Game, s: usize, bh: usize) {
    let (w, h) = (c.w, c.h);
    let fh = h - bh;
    let horizon = fh * 6 / 10;
    gradient(c, 0, 0, w, horizon, 0xFF63B3ED, 0xFFBEE3F8);
    gradient(c, 0, horizon, w, fh - horizon, 0xFF9AE6B4, 0xFF276749);
    let r = fh / 7;
    let (fx, fy) = (w * 3 / 4, fh * 3 / 10);
    let (px, py) = (w / 4, fh * 7 / 10);
    ellipse(c, fx, fy + r + r / 3, r * 3 / 2, (r / 4).max(1), SHADOW);
    ellipse(c, px, py + r + r / 3, r * 3 / 2, (r / 4).max(1), SHADOW);
    mon(c, fx, fy, r, g.battle.foe.sp);
    mon(c, px, py, r, g.party[g.active].sp);
    panel(c, w / 20, fh / 10, w * 2 / 5, &g.battle.foe, s);
    panel(c, w * 11 / 20, fh * 6 / 10, w * 2 / 5, &g.party[g.active], s);

    let my = fh + 3 * s;
    dialog_box(c, 0, fh, w, bh, s, "", false);
    text(c, 3 * s, my, s, WHITE, g.battle.msg.as_str());
    let (mx, names): (usize, [&str; 3]) = (w * 2 / 3 + 3 * s, ["LOTTA", "CATTURA", "FUGGI"]);
    match g.battle.phase {
        Phase::Menu => menu(c, mx, my, s, &names, g.battle.cursor),
        Phase::Moves => {
            let mv = SPECIES[g.party[g.active].sp].moves;
            menu(c, mx, my, s, &[MOVES[mv[0]].name, MOVES[mv[1]].name, ""], g.battle.cursor)
        }
        Phase::Done => text(c, mx, my, s, CURSOR, "A: CONTINUA"),
    }
}

fn menu<P: Pixel>(c: &mut Canvas<P>, x: usize, y: usize, s: usize, items: &[&str], cur: usize) {
    for (i, name) in items.iter().enumerate().filter(|(_, n)| !n.is_empty()) {
        let yy = y + i * 8 * s;
        if i == cur {
            text(c, x, yy, s, CURSOR, ">");
        }
        text(c, x + 5 * s, yy, s, WHITE, name);
    }
}

/// Scatola di dialogo in basso: gradiente, bordo doppio, freccia se ci sono altre pagine.
fn dialog_box<P: Pixel>(c: &mut Canvas<P>, x: usize, y: usize, w: usize, h: usize, s: usize, t: &str, more: bool) {
    gradient(c, x, y, w, h, 0xFF2D3748, 0xFF1A202C);
    c.frame(x + 2, y + 2, w - 4, h - 4, WHITE);
    c.frame(x + 4, y + 4, w - 8, h - 8, GREY);
    text(c, x + 3 * s + 4, y + 3 * s + 4, s, WHITE, t);
    if more {
        let (ax, ay) = (x + w - 6 * s, y + h - 5 * s);
        for i in 0..4 {
            c.rect(ax + i * s, ay + i * s, s * (8 - 2 * i), s, CURSOR);
        }
    }
}

/// Riquadro di stato di una creatura: nome, barra HP.
fn panel<P: Pixel>(c: &mut Canvas<P>, x: usize, y: usize, pw: usize, cr: &Creature, s: usize) {
    let ph = 19 * s;
    let max = cr.s().hp;
    c.rect(x, y, pw, ph, PANEL);
    c.frame(x, y, pw, ph, INK);
    text(c, x + 2 * s, y + 2 * s, s, INK, cr.s().name);
    let bw = pw - 4 * s;
    c.rect(x + 2 * s, y + 8 * s, bw, 3 * s, GREY);
    let col = if cr.hp * 2 > max {
        0xFF48BB78
    } else if cr.hp * 5 > max {
        CURSOR
    } else {
        0xFFE53E3E
    };
    c.rect(x + 2 * s, y + 8 * s, (cr.hp.max(0) * bw as i32 / max) as usize, 3 * s, col);
    let mut t = Msg::new();
    t.push("PS ");
    t.num(cr.hp);
    t.push("/");
    t.num(max);
    text(c, x + 2 * s, y + 13 * s, s, INK, t.as_str());
}

fn kind_color(k: Kind) -> u32 {
    match k {
        Kind::Normal => 0xFFA0AEC0,
        Kind::Fire => 0xFFE53E3E,
        Kind::Water => 0xFF3182CE,
        Kind::Grass => 0xFF38A169,
    }
}

/// Creatura: disco con evidenziatura, occhi con pupille.
fn mon<P: Pixel>(c: &mut Canvas<P>, cx: usize, cy: usize, r: usize, sp: usize) {
    let col = kind_color(SPECIES[sp].kind);
    disc(c, cx, cy, r, col);
    disc(c, cx - r / 3, cy - r / 3, (r / 4).max(1), mix(col, WHITE, 110));
    let (ew, eh) = ((r / 4).max(1), (r / 4).max(1));
    c.rect(cx - r / 2, cy - r / 5, ew, eh, WHITE);
    c.rect(cx + r / 4, cy - r / 5, ew, eh, WHITE);
    c.rect(cx - r / 2 + ew / 3, cy - r / 5 + eh / 3, (ew / 2).max(1), (eh / 2).max(1), INK);
    c.rect(cx + r / 4 + ew / 3, cy - r / 5 + eh / 3, (ew / 2).max(1), (eh / 2).max(1), INK);
}

/// Testo con font 3x5 (solo maiuscole, cifre e punteggiatura). `\n` va a capo.
pub fn text<P: Pixel>(c: &mut Canvas<P>, x: usize, y: usize, s: usize, col: u32, t: &str) {
    let (mut cx, mut cy) = (x, y);
    for ch in t.chars() {
        if ch == '\n' {
            cx = x;
            cy += 6 * s;
            continue;
        }
        for (r, bits) in glyph(ch).iter().enumerate() {
            for k in 0..3 {
                if bits & (4u8 >> k) != 0 {
                    c.rect(cx + k * s, cy + r * s, s, s, col);
                }
            }
        }
        cx += 4 * s;
    }
}

/// Transizione: barre nere che si ritirano verso sinistra, `fade` da 16 a 1.
fn wipe<P: Pixel>(c: &mut Canvas<P>, fade: u32) {
    let (w, h) = (c.w, c.h);
    let bars = 12;
    let bh = h / bars;
    for i in 0..bars {
        let wd = (w * (fade as usize + i % 3) / 16).min(w);
        c.rect(0, i * bh, wd, bh + 1, 0xFF000000);
    }
}

fn glyph(ch: char) -> [u8; 5] {
    match ch {
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b011, 0b100, 0b101, 0b101, 0b011],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b010],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b110, 0b101, 0b101, 0b101, 0b101],
        'O' => [0b010, 0b101, 0b101, 0b101, 0b010],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'Q' => [0b010, 0b101, 0b101, 0b110, 0b011],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b110, 0b001, 0b010, 0b100, 0b111],
        '3' => [0b110, 0b001, 0b010, 0b001, 0b110],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b110, 0b001, 0b110],
        '6' => [0b011, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b110],
        '!' => [0b010, 0b010, 0b010, 0b000, 0b010],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        ',' => [0b000, 0b000, 0b000, 0b010, 0b100],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        '?' => [0b110, 0b001, 0b010, 0b000, 0b010],
        '>' => [0b100, 0b010, 0b001, 0b010, 0b100],
        '\'' => [0b010, 0b010, 0b000, 0b000, 0b000],
        _ => [0; 5],
    }
}

#[cfg(test)]
mod tests {
    use super::glyph;
    use crate::world::NPCS;

    #[test]
    fn npc_texts_have_glyphs() {
        for n in NPCS.iter() {
            for ch in n.text.chars().filter(|&c| c != '|' && c != ' ') {
                assert_ne!(glyph(ch), [0; 5], "missing glyph {ch:?} in {}", n.text);
            }
        }
    }
}
