//! Disegno del gioco su `Canvas`, indipendente dal formato pixel. `no_std`.

use crate::battle::{Creature, Kind, Msg, Phase, MOVES, SPECIES};
use crate::world::{self, NpcKind, NPCS};
use crate::{Game, Mode};
use haiku_editor::gui::{Canvas, Pixel};

const BG: u32 = 0xFF1A202C;
const WHITE: u32 = 0xFFFFFFFF;
const INK: u32 = 0xFF1A202C;
const PANEL: u32 = 0xFFF7FAFC;
const GREY: u32 = 0xFF4A5568;
const CURSOR: u32 = 0xFFECC94B;
const SKIN: u32 = 0xFFFED7AA;

/// Disegna l'intero schermo. La scala del testo dipende dall'altezza.
pub fn draw<P: Pixel>(c: &mut Canvas<P>, g: &Game) {
    let (w, h) = (c.w, c.h);
    let s = (h / 150).max(2);
    let bh = h / 4;
    c.clear(BG);
    if g.mode == Mode::Fight {
        battle_view(c, g, s, bh);
        return;
    }
    let mh = h - bh;
    let ts = (w / 16).min(mh / 12);
    map_view(c, g, (w - 16 * ts) / 2, (mh - 12 * ts) / 2, ts);
    let mut status = Msg::new();
    let body = if g.mode == Mode::Talk {
        g.text.split('|').nth(g.page).unwrap_or("")
    } else {
        status.push(if g.map == 0 { "VERDEPOLI" } else { "PERCORSO 1" });
        status.push("   PALLE ");
        status.num(g.balls as i32);
        status.as_str()
    };
    dialog_box(c, 0, mh, w, bh, s, body);
}

fn map_view<P: Pixel>(c: &mut Canvas<P>, g: &Game, ox: usize, oy: usize, ts: usize) {
    for y in 0..12 {
        let row = world::map(g.map)[y].as_bytes();
        for x in 0..16 {
            let ch = row[x] as char;
            let (px, py) = (ox + x * ts, oy + y * ts);
            c.rect(px, py, ts, ts, tile_color(ch));
            if ch == ',' {
                c.rect(px + ts / 4, py + ts / 3, ts / 2, (ts / 8).max(1), 0xFF276749);
            }
        }
    }
    for n in NPCS.iter().filter(|n| n.map == g.map) {
        let col = match n.kind {
            NpcKind::Nurse => WHITE,
            NpcKind::Trainer => 0xFF805AD5,
            NpcKind::Talk => CURSOR,
        };
        actor(c, ox + n.x as usize * ts, oy + n.y as usize * ts, ts, col);
    }
    actor(c, ox + g.x as usize * ts, oy + g.y as usize * ts, ts, 0xFFE53E3E);
}

fn tile_color(ch: char) -> u32 {
    match ch {
        '#' => 0xFF2D3748,
        'T' => 0xFF22543D,
        'H' => 0xFF9C4221,
        ',' => 0xFF68D391,
        '=' | '<' | '>' => 0xFFE2C78F,
        _ => 0xFF9AE6B4,
    }
}

/// Personaggio su tile: corpo colorato e faccia.
fn actor<P: Pixel>(c: &mut Canvas<P>, x: usize, y: usize, ts: usize, col: u32) {
    c.rect(x + ts / 5, y + ts / 5, ts * 3 / 5, ts * 3 / 5, col);
    c.rect(x + ts * 3 / 10, y + ts * 2 / 5, ts * 2 / 5, ts / 5, SKIN);
}

fn battle_view<P: Pixel>(c: &mut Canvas<P>, g: &Game, s: usize, bh: usize) {
    let (w, h) = (c.w, c.h);
    let fh = h - bh;
    c.rect(0, 0, w, fh, 0xFFC6F6D5);
    c.rect(0, fh * 6 / 10, w, fh * 4 / 10, 0xFF9AE6B4);
    let r = fh / 7;
    mon(c, w * 3 / 4, fh * 3 / 10, r, g.battle.foe.sp);
    mon(c, w / 4, fh * 7 / 10, r, g.party[g.active].sp);
    panel(c, w / 20, fh / 10, w * 2 / 5, &g.battle.foe, s);
    panel(c, w * 11 / 20, fh * 6 / 10, w * 2 / 5, &g.party[g.active], s);

    let my = fh + 3 * s;
    dialog_box(c, 0, fh, w, bh, s, "");
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

/// Scatola di dialogo in basso con bordo bianco.
fn dialog_box<P: Pixel>(c: &mut Canvas<P>, x: usize, y: usize, w: usize, h: usize, s: usize, t: &str) {
    c.rect(x, y, w, h, BG);
    c.frame(x + 2, y + 2, w - 4, h - 4, WHITE);
    text(c, x + 3 * s + 4, y + 3 * s + 4, s, WHITE, t);
}

/// Riquadro di stato di una creatura: nome e barra HP.
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
    t.push("HP ");
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

/// Creatura come disco colorato per tipo, con occhi.
fn mon<P: Pixel>(c: &mut Canvas<P>, cx: usize, cy: usize, r: usize, sp: usize) {
    disc(c, cx, cy, r, kind_color(SPECIES[sp].kind));
    c.rect(cx - r / 2, cy - r / 4, r / 4, r / 4, WHITE);
    c.rect(cx + r / 4, cy - r / 4, r / 4, r / 4, WHITE);
}

fn disc<P: Pixel>(c: &mut Canvas<P>, cx: usize, cy: usize, r: usize, col: u32) {
    let (ri, r2) = (r as isize, (r * r) as isize);
    for dy in -ri..=ri {
        let mut hw = ri;
        while hw * hw + dy * dy > r2 {
            hw -= 1;
        }
        c.rect(cx - hw as usize, (cy as isize + dy) as usize, (2 * hw + 1) as usize, 1, col);
    }
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
