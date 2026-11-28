//! Rendering vettoriale su framebuffer ARGB. Ogni oggetto è un insieme di poligoni, archi ed ellissi
//! espressi in unità di 1/16 px logico (vista VIEW_W×VIEW_H). La rasterizzazione avviene alla
//! risoluzione di uscita, con antialiasing a 4 sotto-righe. Legge lo stato, non lo modifica.
use crate::levels::LEVELS;
use crate::*;
use haiku_gui::layout::Rect;
use haiku_gui::palette::Tok;
use haiku_gui::render::Canvas;
use haiku_gui::vfont;

/// Larghezza massima del buffer (px fisici).
const MAXW: usize = 1920;
/// Coseno e seno a 16 passi (×256), da 0°, in senso orario sullo schermo.
const COS: [i32; 16] = [256, 236, 181, 98, 0, -98, -181, -236, -256, -236, -181, -98, 0, 98, 181, 236];
const SIN: [i32; 16] = [0, 98, 181, 236, 256, 236, 181, 98, 0, -98, -181, -236, -256, -236, -181, -98];

const SKY_TOP: u32 = 0xFF5FB4F0;
const SKY_BOT: u32 = 0xFFD0F0FF;
const HILL: u32 = 0xFF4E9F3D;
const HILL_FAR: u32 = 0xFF3F8A32;
const CLOUD: u32 = 0xFFF8FCFF;
const GRASS: u32 = 0xFF5BC142;
const DIRT: u32 = 0xFF8B5A2B;
const SEAM: u32 = 0xFF6E4420;
const BRICK: u32 = 0xFFB5523B;
const MORTAR: u32 = 0xFF7A3322;
const QUEST: u32 = 0xFFF2B233;
const QUEST_IN: u32 = 0xFFFFD76A;
const INK: u32 = 0xFF2A1A0A;
const USED: u32 = 0xFF6E5A4A;
const COIN: u32 = 0xFFFFD700;
const COIN_IN: u32 = 0xFFFFF07A;
const FLAG: u32 = 0xFF2E8B57;
const POLE: u32 = 0xFFEEEEEE;
const FUNGO_CAP: u32 = 0xFF9C5A2E;
const FUNGO_BODY: u32 = 0xFFF3D9A4;
const MUSH_CAP: u32 = 0xFFE03C31;
const MUSH_STEM: u32 = 0xFFF5E6C8;
const STAR_C: u32 = 0xFFFFE94D;
const WHITE: u32 = 0xFFFFFFFF;
const BLACK: u32 = 0xFF111111;
const CAP: u32 = 0xFFD83A2A;
const SHIRT: u32 = 0xFFD83A2A;
const OVERALL: u32 = 0xFF2A5FD8;
const SKIN: u32 = 0xFFF2C29B;
const SHOE: u32 = 0xFF5A2D0C;
const MOUSTACHE: u32 = 0xFF4A2A10;
const HUD_BG: u32 = 0xFF101820;
const PATH: u32 = 0xFF7A5B3A;
const NODE_ON: u32 = 0xFF4CAF50;
const NODE_OFF: u32 = 0xFF5B6470;
const TEXT_SZ: i32 = 10;

/// Logico intero → unità di 1/16 px.
pub const fn p(v: i32) -> i32 {
    v * 16
}

fn argb(t: Tok) -> u32 {
    0xFF00_0000 | t.rgb()
}

fn isqrt(n: i64) -> i64 {
    if n < 2 {
        return n;
    }
    let (mut x, mut y) = (n, (n + 1) / 2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Framebuffer con mappa logico→device: la vista logica è centrata e adattata, bordi neri.
pub struct Fb<'a> {
    buf: &'a mut [u32],
    w: i32,
    h: i32,
    /// Px device per px logico, in unità 1/65536.
    sc: i64,
    ox: i32,
    oy: i32,
    /// Copertura della riga corrente (riusata, si azzera a fine riga).
    cov: [u16; MAXW],
}

impl<'a> Fb<'a> {
    pub fn new(buf: &'a mut [u32], w: usize, h: usize) -> Fb<'a> {
        assert!(w <= MAXW && buf.len() == w * h);
        let (wi, hi) = (w as i64, h as i64);
        let sc = (wi * 65536 / VIEW_W as i64).min(hi * 65536 / VIEW_H as i64);
        let ox = (wi - VIEW_W as i64 * sc / 65536) / 2;
        let oy = (hi - VIEW_H as i64 * sc / 65536) / 2;
        Fb { buf, w: w as i32, h: h as i32, sc, ox: ox as i32, oy: oy as i32, cov: [0; MAXW] }
    }

    pub fn clear(&mut self, c: u32) {
        self.buf.iter_mut().for_each(|p| *p = c);
    }

    /// Pulisce i bordi fuori dalla vista logica (sfondo e colline non escono).
    pub fn bars(&mut self) {
        let x1 = self.ox + (VIEW_W as i64 * self.sc / 65536) as i32;
        let y1 = self.oy + (VIEW_H as i64 * self.sc / 65536) as i32;
        let (w, h) = (self.w, self.h);
        self.fill_dev(0, 0, self.ox, h, 0xFF000000);
        self.fill_dev(x1, 0, w, h, 0xFF000000);
        self.fill_dev(0, 0, w, self.oy, 0xFF000000);
        self.fill_dev(0, y1, w, h, 0xFF000000);
    }

    /// Attenua tutto il fotogramma (sfondo dei menu): metà luminosità.
    pub fn darken(&mut self) {
        self.buf.iter_mut().for_each(|p| *p = 0xFF00_0000 | ((*p >> 1) & 0x007F_7F7F));
    }

    /// Logico in unità 1/16 px → device in unità 1/16 px.
    fn q(&self, x: i32, y: i32) -> (i32, i32) {
        (
            (self.ox as i64 * 16 + x as i64 * self.sc / 65536) as i32,
            (self.oy as i64 * 16 + y as i64 * self.sc / 65536) as i32,
        )
    }

    /// Logico intero → device intero (per difetto: i bordi di tile e rettangoli combaciano).
    fn d(&self, x: i32, y: i32) -> (i32, i32) {
        let (a, b) = self.q(x * 16, y * 16);
        (a >> 4, b >> 4)
    }

    fn fill_dev(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u32) {
        for y in y0.max(0)..y1.min(self.h) {
            let row = (y * self.w) as usize;
            for x in x0.max(0)..x1.min(self.w) {
                self.buf[row + x as usize] = c;
            }
        }
    }

    /// Mescola il colore `c` sul pixel device (x, y) con copertura `a` (0..=255).
    fn px(&mut self, x: i32, y: i32, a: u32, c: u32) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h || a == 0 {
            return;
        }
        let i = (y * self.w + x) as usize;
        if a >= 255 {
            self.buf[i] = c;
            return;
        }
        let d = self.buf[i];
        let mix = |sh: u32| (((c >> sh) & 255) * a + ((d >> sh) & 255) * (255 - a)) / 255 << sh;
        self.buf[i] = 0xFF00_0000 | mix(16) | mix(8) | mix(0);
    }

    /// Rettangolo logico intero pieno.
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        let (x0, y0) = self.d(x, y);
        let (x1, y1) = self.d(x + w, y + h);
        self.fill_dev(x0, y0, x1, y1, c);
    }

    /// Poligono pieno (regola pari) con antialiasing; vertici in unità 1/16 logiche.
    pub fn poly(&mut self, pts: &[(i32, i32)], c: u32) {
        let mut v = [(0i32, 0i32); 40];
        let n = pts.len().min(v.len());
        for i in 0..n {
            v[i] = self.q(pts[i].0, pts[i].1);
        }
        self.poly_dev(&v[..n], c);
    }

    fn poly_dev(&mut self, v: &[(i32, i32)], c: u32) {
        let n = v.len();
        if n < 3 {
            return;
        }
        let ymin = v.iter().map(|q| q.1).min().unwrap_or(0);
        let ymax = v.iter().map(|q| q.1).max().unwrap_or(0);
        let (w16, wd) = (self.w * 16, self.w);
        for row in (ymin >> 4).max(0)..((ymax + 15) >> 4).min(self.h) {
            let (mut lo, mut hi) = (wd, -1);
            for k in 0..4 {
                let sy = row * 16 + 2 + k * 4;
                let mut xs = [0i32; 32];
                let mut m = 0;
                for i in 0..n {
                    let (a, b) = (v[i], v[(i + 1) % n]);
                    if (a.1 <= sy) != (b.1 <= sy) {
                        let x = a.0 as i64 + (sy - a.1) as i64 * (b.0 - a.0) as i64 / (b.1 - a.1) as i64;
                        if m < xs.len() {
                            xs[m] = x as i32;
                            m += 1;
                        }
                    }
                }
                xs[..m].sort_unstable();
                for j in (0..m & !1).step_by(2) {
                    let (xl, xr) = (xs[j].max(0), xs[j + 1].min(w16));
                    if xr <= xl {
                        continue;
                    }
                    for px in (xl >> 4)..=((xr - 1) >> 4) {
                        let ov = xr.min(px * 16 + 16) - xl.max(px * 16);
                        self.cov[px as usize] += (ov * 4) as u16;
                        lo = lo.min(px);
                        hi = hi.max(px);
                    }
                }
            }
            for px in lo..=hi {
                let a = self.cov[px as usize].min(255) as u32;
                self.cov[px as usize] = 0;
                self.px(px, row, a, c);
            }
        }
    }

    /// Ellisse con centro (cx, cy) e raggi (rx, ry); archi da indice `a0` ad `a1` (0..16, un passo = 22.5°).
    pub fn arc(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, a0: i32, a1: i32, c: u32) {
        let mut v = [(0i32, 0i32); 17];
        let mut n = 0;
        for i in a0..=a1 {
            let k = (i & 15) as usize;
            v[n] = (cx + rx * COS[k] / 256, cy + ry * SIN[k] / 256);
            n += 1;
        }
        self.poly(&v[..n], c);
    }

    pub fn disc(&mut self, cx: i32, cy: i32, r: i32, c: u32) {
        self.arc(cx, cy, r, r, 0, 15, c);
    }

    pub fn ell(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, c: u32) {
        self.arc(cx, cy, rx, ry, 0, 15, c);
    }

    /// Settore d'anello fra raggi `ro` (esterno) e `ri` (interno), archi `a0..=a1`.
    pub fn ring(&mut self, cx: i32, cy: i32, ro: i32, ri: i32, a0: i32, a1: i32, c: u32) {
        let mut v = [(0i32, 0i32); 34];
        let mut n = 0;
        for i in a0..=a1 {
            let k = (i & 15) as usize;
            v[n] = (cx + ro * COS[k] / 256, cy + ro * SIN[k] / 256);
            n += 1;
        }
        for i in (a0..=a1).rev() {
            let k = (i & 15) as usize;
            v[n] = (cx + ri * COS[k] / 256, cy + ri * SIN[k] / 256);
            n += 1;
        }
        self.poly(&v[..n], c);
    }

    /// Rettangolo con angoli arrotondati di raggio `r` (unità 1/16).
    pub fn rrect(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, c: u32) {
        let mut v = [(0i32, 0i32); 20];
        let mut n = 0;
        for (cx, cy, a0) in [(x + r, y + r, 8), (x + w - r, y + r, 12), (x + w - r, y + h - r, 0), (x + r, y + h - r, 4)] {
            for k in 0..=4 {
                let i = ((a0 + k) & 15) as usize;
                v[n] = (cx + r * COS[i] / 256, cy + r * SIN[i] / 256);
                n += 1;
            }
        }
        self.poly(&v, c);
    }

    /// Segmento di spessore `w` (unità 1/16) come quadrilatero.
    pub fn seg(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, w: i32, c: u32) {
        let (dx, dy) = ((x2 - x1) as i64, (y2 - y1) as i64);
        let len = isqrt(dx * dx + dy * dy).max(1);
        let (nx, ny) = ((-dy * w as i64 / 2 / len) as i32, (dx * w as i64 / 2 / len) as i32);
        self.poly(&[(x1 + nx, y1 + ny), (x2 + nx, y2 + ny), (x2 - nx, y2 - ny), (x1 - nx, y1 - ny)], c);
    }
}

/// Adattatore: `vfont` scrive i pixel direttamente alle coordinate device.
struct Dev<'a, 'b>(&'a mut Fb<'b>);

impl Canvas for Dev<'_, '_> {
    fn fill(&mut self, r: Rect, t: Tok) {
        self.0.fill_dev(r.x, r.y, r.x + r.w, r.y + r.h, argb(t));
    }
    fn text(&mut self, _x: i32, _y: i32, _s: &str, _t: Tok) {}
    fn blend(&mut self, x: i32, y: i32, a: u8, t: Tok) {
        self.0.px(x, y, a as u32, argb(t));
    }
}

impl Canvas for Fb<'_> {
    fn fill(&mut self, r: Rect, t: Tok) {
        self.rect(r.x, r.y, r.w, r.h, argb(t));
    }
    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok) {
        self.vtext(x, y, TEXT_SZ, s, t);
    }
    fn smooth(&self) -> bool {
        true
    }
    fn rect_rounded(&mut self, r: Rect, rad: i32, t: Tok) {
        self.rrect(p(r.x), p(r.y), p(r.w), p(r.h), p(rad), argb(t));
    }
    fn poly(&mut self, pts: &[(i32, i32)], t: Tok) {
        let mut v = [(0i32, 0i32); 40];
        let n = pts.len().min(v.len());
        for i in 0..n {
            v[i] = (p(pts[i].0), p(pts[i].1));
        }
        Fb::poly(self, &v[..n], argb(t));
    }
    /// Testo vettoriale a `size` px logici: i contorni sono rasterizzati alla risoluzione di uscita.
    fn vtext(&mut self, x: i32, y: i32, size: i32, s: &str, t: Tok) -> i32
    where
        Self: Sized,
    {
        let (dx, dy) = self.d(x, y);
        let ds = (size as i64 * self.sc / 65536) as i32;
        let w = vfont::draw_text(&mut Dev(&mut *self), dx, dy, ds, s, t);
        (w as i64 * 65536 / self.sc) as i32
    }
}

/// Disegna lo stato corrente di `g` nel buffer `w`×`h` (ARGB).
pub fn draw(g: &Game, buf: &mut [u32], w: usize, h: usize) {
    let mut c = Fb::new(buf, w, h);
    c.clear(0xFF000000);
    if g.state == State::Map {
        return draw_map(g, &mut c);
    }
    world(g, &mut c);
    c.bars();
    hud(g, &mut c);
    match g.state {
        State::Clear => banner(&mut c, FLAG, "LIVELLO SUPERATO"),
        State::Over => banner(&mut c, MUSH_CAP, "GAME OVER"),
        State::Won => banner(&mut c, STAR_C, "VITTORIA"),
        _ => {}
    }
}

fn lerp(a: u32, b: u32, t: u32) -> u32 {
    let ch = |s: u32| ((a >> s & 255) * (255 - t) + (b >> s & 255) * t) / 255 << s;
    0xFF00_0000 | ch(16) | ch(8) | ch(0)
}

fn sky(c: &mut Fb) {
    for i in 0..6 {
        c.rect(0, i * 40, VIEW_W, 40, lerp(SKY_TOP, SKY_BOT, i as u32 * 51));
    }
}

/// Sfondo: cielo, colline e nuvole in parallasse rispetto alla camera.
fn backdrop(c: &mut Fb, cam: i32) {
    sky(c);
    for k in -1..6 {
        let x = k * 120 + 60 - (cam / 2).rem_euclid(120);
        c.ell(p(x), p(200), p(70), p(48), HILL_FAR);
    }
    for k in -1..5 {
        let x = k * 100 + 30 - (cam / 3).rem_euclid(100);
        c.ell(p(x), p(206), p(50), p(36), HILL);
    }
    for k in 0..5 {
        let x = (k * 90 - cam * 3 / 10).rem_euclid(VIEW_W + 120) - 60;
        let y = 28 + (k % 3) * 18;
        c.ell(p(x), p(y + 2), p(20), p(5), CLOUD);
        c.disc(p(x + 9), p(y - 3), p(9), CLOUD);
        c.disc(p(x + 22), p(y), p(7), CLOUD);
    }
}

fn world(g: &Game, c: &mut Fb) {
    let cam = g.cam_x;
    backdrop(c, cam);
    tiles(g, c);
    for m in g.mobs.iter().filter(|m| m.alive) {
        let (x, y) = (m.b.x / SUB - cam, m.b.y / SUB);
        match m.kind {
            MobKind::Fungo => fungo(c, x, y, (g.frame / 8 % 2) as i32),
            MobKind::Mushroom => mushroom(c, x, y),
            MobKind::Star => star(c, p(x + 7), p(y + 7), p(7), (g.frame / 3 % 16) as i32),
        }
    }
    player(g, c);
}

fn tiles(g: &Game, c: &mut Fb) {
    let cam = g.cam_x;
    let lvl = &g.lvl;
    let tx0 = cam / TILE;
    let tx1 = ((cam + VIEW_W) / TILE + 1).min(lvl.w as i32);
    for ty in 0..lvl.h as i32 {
        for tx in tx0..tx1 {
            let (x, y) = (tx * TILE - cam, ty * TILE);
            match lvl.tile(tx, ty) {
                Tile::Ground => {
                    c.rect(x, y, TILE, TILE, DIRT);
                    if !lvl.solid(tx, ty - 1) {
                        c.rect(x, y, TILE, 3, GRASS);
                    }
                    c.rect(x, y + 9, TILE, 1, SEAM);
                    c.rect(x + 8, y + 10, 1, 6, SEAM);
                }
                Tile::Brick => {
                    c.rect(x, y, TILE, TILE, BRICK);
                    c.rect(x, y + 7, TILE, 1, MORTAR);
                    c.rect(x + 7, y, 1, 7, MORTAR);
                    c.rect(x + 3, y + 8, 1, 8, MORTAR);
                    c.rect(x + 11, y + 8, 1, 8, MORTAR);
                }
                Tile::Question(_) => {
                    c.rrect(p(x), p(y), p(16), p(16), p(2), QUEST);
                    c.rrect(p(x + 2), p(y + 2), p(12), p(12), p(1), QUEST_IN);
                    c.ring(p(x + 8), p(y + 6), p(3) + 8, p(2), 8, 16, INK);
                    c.seg(p(x + 8), p(y + 6), p(x + 8), p(y + 10), p(2), INK);
                    c.disc(p(x + 8), p(y + 12) + 4, p(1) + 4, INK);
                }
                Tile::Used => {
                    c.rrect(p(x), p(y), p(16), p(16), p(2), USED);
                    for (dx, dy) in [(3, 3), (12, 3), (3, 12), (12, 12)] {
                        c.disc(p(x + dx), p(y + dy), p(1), INK);
                    }
                }
                Tile::Coin => coin(c, p(x + 8), p(y + 8), g.frame),
                Tile::Flag => {
                    c.rect(x + 7, y, 2, TILE, POLE);
                    c.poly(&[(x + 9, y + 1), (x + 16, y + 4), (x + 9, y + 7)], FLAG);
                    c.disc(p(x + 8), p(y), p(1), POLE);
                }
                Tile::Air => {}
            }
        }
    }
}

/// Moneta che ruota: larghezza proporzionale al coseno.
fn coin(c: &mut Fb, cx: i32, cy: i32, frame: u32) {
    let k = (frame as usize * 2) % 16;
    let rx = (p(5) * COS[k].abs() / 256).max(p(1) / 2);
    c.ell(cx, cy, rx, p(5), COIN);
    if rx > p(2) {
        c.ell(cx, cy, rx - p(2) / 2, p(3), COIN_IN);
    }
}

fn fungo(c: &mut Fb, x: i32, y: i32, f: i32) {
    let (x, y) = (p(x), p(y));
    c.arc(x + p(7), y + p(7), p(7), p(7), 8, 16, FUNGO_CAP);
    c.rrect(x + p(1), y + p(7), p(12), p(6), p(2), FUNGO_BODY);
    c.disc(x + p(4), y + p(9), p(2) + 8, WHITE);
    c.disc(x + p(10), y + p(9), p(2) + 8, WHITE);
    c.disc(x + p(4) + p(1) / 2, y + p(9), p(1), BLACK);
    c.disc(x + p(10) + p(1) / 2, y + p(9), p(1), BLACK);
    c.rrect(x + p(1 + f * 2), y + p(12), p(4), p(2), p(1), INK);
    c.rrect(x + p(9 - f * 2), y + p(12), p(4), p(2), p(1), INK);
}

fn mushroom(c: &mut Fb, x: i32, y: i32) {
    let (x, y) = (p(x), p(y));
    c.rrect(x + p(3), y + p(7), p(8), p(7), p(2), MUSH_STEM);
    c.arc(x + p(7), y + p(7), p(7), p(7), 8, 16, MUSH_CAP);
    c.disc(x + p(4), y + p(4), p(1) + 8, WHITE);
    c.disc(x + p(10), y + p(4), p(1) + 8, WHITE);
    c.disc(x + p(7), y + p(1) + 8, p(1), WHITE);
}

/// Stella a 8 punte che ruota; `off` (0..16) è la fase.
fn star(c: &mut Fb, cx: i32, cy: i32, r: i32, off: i32) {
    let mut v = [(0i32, 0i32); 16];
    for k in 0..16 {
        let i = ((off + k) & 15) as usize;
        let rr = if k % 2 == 0 { r } else { r * 45 / 100 };
        v[k as usize] = (cx + rr * COS[i] / 256, cy + rr * SIN[i] / 256);
    }
    c.poly(&v, STAR_C);
    c.disc(cx, cy, r / 4, WHITE);
}

/// Personaggio: sagoma composta da forme; si allunga in salto, cammina a passi.
fn player(g: &Game, c: &mut Fb) {
    let pl = &g.p;
    if pl.hurt > 0 && (g.frame / 4) % 2 == 0 {
        return;
    }
    let bx = pl.b.x / SUB - g.cam_x;
    let (by, bh) = (pl.b.y / SUB, pl.b.h);
    let moving = pl.b.vx.abs() > 4;
    let s = if !pl.on_ground {
        2
    } else if moving {
        (g.frame / 4 % 2) as i32
    } else {
        0
    };
    let (cap, shirt, overall) = if pl.star > 0 {
        let k = [CAP, STAR_C, OVERALL][(g.frame / 3 % 3) as usize];
        (k, k, k)
    } else {
        (CAP, SHIRT, OVERALL)
    };
    let f = pl.facing;
    // v: unità 1/16 nella sagoma base alta 14 px; la verticale scala con bh (14 o 24).
    let x = |v: i32| p(bx) + v;
    let y = |v: i32| p(by) + v * bh / 14;
    let h = |v: i32| v * bh / 14;
    // Gambe e scarpe.
    c.rrect(x(p(1) + p(s)), y(p(12)), p(4), h(p(2)), p(1), SHOE);
    c.rrect(x(p(7) - p(s)), y(p(12)), p(4), h(p(2)), p(1), SHOE);
    // Salopette e maglia.
    c.rrect(x(p(2)), y(p(7)), p(8), h(p(6)), p(2), overall);
    c.rrect(x(p(2)), y(p(5)), p(8), h(p(3)), p(2), shirt);
    // Testa, berretto e visiera sul lato dello sguardo.
    c.disc(x(p(6)), y(p(4)), p(3) + 8, SKIN);
    c.arc(x(p(6)), y(p(4)), p(4), p(4), 8, 16, cap);
    let vis = if f > 0 { p(6) } else { p(2) };
    c.rrect(x(vis), y(p(4)), p(5), p(1), p(1) / 2, cap);
    let ex = if f > 0 { p(8) } else { p(4) };
    c.disc(x(ex), y(p(5)), p(1) + 8, WHITE);
    c.disc(x(ex + p(f) / 2), y(p(5)), p(1) / 2 + 2, BLACK);
    c.rrect(x(p(4)), y(p(6)), p(4), h(p(1)), p(1) / 2, MOUSTACHE);
}

/// HUD: barra superiore con punteggio, monete e vite.
fn hud(g: &Game, c: &mut Fb) {
    let mut b = [0u8; 10];
    c.rect(0, 0, VIEW_W, 14, HUD_BG);
    c.text(4, 2, num_str(g.score, &mut b), Tok::T1);
    c.disc(p(114), p(7), p(3) + 8, COIN);
    c.text(122, 2, num_str(g.coins, &mut b), Tok::T1);
    c.disc(p(185), p(7), p(4), CAP);
    c.text(194, 2, num_str(g.lives as u32, &mut b), Tok::T1);
}

fn banner(c: &mut Fb, accent: u32, msg: &str) {
    c.rrect(p(40), p(96), p(176), p(48), p(6), HUD_BG);
    c.rrect(p(40), p(96), p(176), p(4), p(2), accent);
    c.vtext(52, 110, 12, msg, Tok::T1);
}

/// Cifre decimali in `buf`, senza allocare: restituisce la parte usata.
pub fn num_str(mut n: u32, buf: &mut [u8; 10]) -> &str {
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    core::str::from_utf8(&buf[i..]).unwrap_or("")
}

fn draw_map(g: &Game, c: &mut Fb) {
    backdrop(c, 0);
    let mut b = [0u8; 10];
    for i in 0..LEVELS.len() {
        let (cx, cy) = (52 + i as i32 * 80, 112);
        if i + 1 < LEVELS.len() {
            c.seg(p(cx), p(cy), p(cx + 80), p(cy), p(4), PATH);
        }
        if i == g.cursor {
            c.disc(p(cx), p(cy), p(15), WHITE);
        }
        let col = if i < g.unlocked { NODE_ON } else { NODE_OFF };
        c.disc(p(cx), p(cy), p(12), col);
        c.text(cx - 3, cy + 18, num_str(i as u32 + 1, &mut b), Tok::T1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vista_centrata_con_bordi_neri() {
        let mut buf = vec![0u32; 64 * 48];
        let mut c = Fb::new(&mut buf, 64, 48);
        c.rect(0, 0, VIEW_W, VIEW_H, 0xFFFFFFFF);
        assert_eq!(buf[32 + 24 * 64], 0xFFFFFFFF);
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn disco_al_centro_della_vista() {
        let mut buf = vec![0u32; 256 * 240];
        let mut c = Fb::new(&mut buf, 256, 240);
        c.disc(p(128), p(120), p(40), 0xFFFFFFFF);
        assert_eq!(buf[120 * 256 + 128], 0xFFFFFFFF);
        assert_eq!(buf[0], 0);
    }
}
