//! Render vettoriale: forme con antialiasing calcolate per pixel, senza sqrt.
//! Coordinate logiche -> pixel con scala uniforme (letterbox).

use crate::{
    Game, Phase, BALL_R, BGX, BGY, BH, BW, BX0, BY0, COLS, CRACKED, EMPTY, NORMAL, PAD_H, PAD_Y, PH, PW, ROWS, SUB,
};

const BG_T: u32 = 0xFF0E1433;
const BG_B: u32 = 0xFF2B1040;
const FRAME_T: u32 = 0xFF3A4D80;
const FRAME_B: u32 = 0xFF1A2448;
const FIELD_T: u32 = 0xFF080B1E;
const FIELD_B: u32 = 0xFF151A3A;
const ROW: [u32; ROWS] = [0xFFE63946, 0xFFF4A261, 0xFFE9C46A, 0xFF2A9D8F, 0xFF457B9D, 0xFF9B5DE5];
const STEEL_T: u32 = 0xFFD5DEEA;
const STEEL_B: u32 = 0xFF6B7789;
const CRACK_T: u32 = 0xFF8C97A8;
const CRACK_B: u32 = 0xFF3A4454;
const CRACK_LINE: u32 = 0xFF1C2230;
const PAD_T: u32 = 0xFFF4F7FF;
const PAD_B: u32 = 0xFF5B6BF0;
const BALL: u32 = 0xFFFFFFFF;
const DROP_T: u32 = 0xFFFFE08A;
const DROP_B: u32 = 0xFFF28F3B;
const TXT: u32 = 0xFFFFFFFF;
const PANEL: u32 = 0xFF0B1030;
const PANEL_EDGE: u32 = 0xFF5B6BF0;

/// Lettere maiuscole 3x5 (bit 2 = colonna sinistra).
const FONT: [[u8; 5]; 26] = [
    [0b010, 0b101, 0b111, 0b101, 0b101], // A
    [0b110, 0b101, 0b110, 0b101, 0b110], // B
    [0b011, 0b100, 0b100, 0b100, 0b011], // C
    [0b110, 0b101, 0b101, 0b101, 0b110], // D
    [0b111, 0b100, 0b110, 0b100, 0b111], // E
    [0b111, 0b100, 0b110, 0b100, 0b100], // F
    [0b011, 0b100, 0b101, 0b101, 0b011], // G
    [0b101, 0b101, 0b111, 0b101, 0b101], // H
    [0b111, 0b010, 0b010, 0b010, 0b111], // I
    [0b001, 0b001, 0b001, 0b101, 0b010], // J
    [0b101, 0b101, 0b110, 0b101, 0b101], // K
    [0b100, 0b100, 0b100, 0b100, 0b111], // L
    [0b101, 0b111, 0b111, 0b101, 0b101], // M
    [0b101, 0b111, 0b111, 0b111, 0b101], // N
    [0b010, 0b101, 0b101, 0b101, 0b010], // O
    [0b110, 0b101, 0b110, 0b100, 0b100], // P
    [0b010, 0b101, 0b101, 0b111, 0b011], // Q
    [0b110, 0b101, 0b110, 0b101, 0b101], // R
    [0b011, 0b100, 0b010, 0b001, 0b110], // S
    [0b111, 0b010, 0b010, 0b010, 0b010], // T
    [0b101, 0b101, 0b101, 0b101, 0b111], // U
    [0b101, 0b101, 0b101, 0b101, 0b010], // V
    [0b101, 0b101, 0b111, 0b111, 0b101], // W
    [0b101, 0b101, 0b010, 0b101, 0b101], // X
    [0b101, 0b101, 0b010, 0b010, 0b010], // Y
    [0b111, 0b001, 0b010, 0b100, 0b111], // Z
];

/// Cifre 0-9 3x5.
const DIG: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b010, 0b010, 0b010],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
];

fn glyph(c: u8) -> [u8; 5] {
    match c {
        b'A'..=b'Z' => FONT[(c - b'A') as usize],
        b'0'..=b'9' => DIG[(c - b'0') as usize],
        _ => [0; 5],
    }
}

fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

fn clamp_f(v: f32, lo: f32, hi: f32) -> f32 {
    v.clamp(lo, hi)
}

#[allow(clippy::too_many_arguments)]
/// Copertura di un rettangolo arrotondato in (px, py). Approssimazione lineare della distanza.
fn rr_cov(px: f32, py: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> f32 {
    let r = if r < 0.5 { 0.5 } else { r };
    let cx = clamp_f(px, x0 + r, x1 - r);
    let cy = clamp_f(py, y0 + r, y1 - r);
    let (dx, dy) = (px - cx, py - cy);
    clamp01(0.5 + (r * r - dx * dx - dy * dy) / (2.0 * r))
}

fn mix(d: u32, s: u32, a: f32) -> u32 {
    let ch = |sh: u32| -> u32 {
        let dc = ((d >> sh) & 255) as f32;
        let sc = ((s >> sh) & 255) as f32;
        (dc + (sc - dc) * a) as u32
    };
    0xFF00_0000 | (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

fn lerp(a: u32, b: u32, t: f32) -> u32 {
    let ch = |sh: u32| -> f32 {
        let (x, y) = (((a >> sh) & 255) as f32, ((b >> sh) & 255) as f32);
        x + (y - x) * t
    };
    0xFF00_0000 | ((ch(16) as u32) << 16) | ((ch(8) as u32) << 8) | (ch(0) as u32)
}

fn shade(c: u32) -> u32 {
    let f = |sh: u32| -> u32 { ((c >> sh) & 255) * 3 / 4 };
    0xFF00_0000 | (f(16) << 16) | (f(8) << 8) | f(0)
}

/// Converte un numero in testo senza allocazione.
fn num(mut n: u32, buf: &mut [u8; 10]) -> &[u8] {
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 || i == 0 {
            break;
        }
    }
    &buf[i..]
}

pub struct Cv<'a> {
    px: &'a mut [u32],
    w: usize,
    h: usize,
    s: f32,
    ox: f32,
    oy: f32,
}

impl<'a> Cv<'a> {
    pub fn new(px: &'a mut [u32], w: usize, h: usize) -> Self {
        let sx = w as f32 / PW as f32;
        let sy = h as f32 / PH as f32;
        let s = if sx < sy { sx } else { sy };
        Cv { px, w, h, s, ox: (w as f32 - PW as f32 * s) / 2.0, oy: (h as f32 - PH as f32 * s) / 2.0 }
    }

    /// Intervallo di pixel che copre [a, b] logico, con un pixel di margine.
    fn span(&self, a: f32, b: f32, o: f32, max: usize) -> (usize, usize) {
        let lo = o + a * self.s - 1.0;
        let hi = o + b * self.s + 1.0;
        let lo = if lo < 0.0 { 0 } else { lo as usize };
        let hi = if hi < 0.0 { 0 } else { hi as usize + 1 };
        (lo.min(max), hi.min(max))
    }

    fn bg(&mut self) {
        for yi in 0..self.h {
            let c = lerp(BG_T, BG_B, yi as f32 / self.h as f32);
            for v in &mut self.px[yi * self.w..(yi + 1) * self.w] {
                *v = c;
            }
        }
    }

    /// Rettangolo arrotondato (coordinate logiche), gradiente verticale top->bot.
    #[allow(clippy::too_many_arguments)]
    pub fn rr(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, r: f32, top: u32, bot: u32) {
        let (dx0, dx1) = self.span(x0, x1, self.ox, self.w);
        let (dy0, dy1) = self.span(y0, y1, self.oy, self.h);
        for yi in dy0..dy1 {
            let ly = ((yi as f32) + 0.5 - self.oy) / self.s;
            let col = lerp(top, bot, clamp01((ly - y0) / (y1 - y0)));
            for xi in dx0..dx1 {
                let lx = ((xi as f32) + 0.5 - self.ox) / self.s;
                let a = rr_cov(lx, ly, x0, y0, x1, y1, r);
                if a > 0.0 {
                    let i = yi * self.w + xi;
                    self.px[i] = mix(self.px[i], col, a);
                }
            }
        }
    }

    /// Cerchio antialiasato (coordinate logiche).
    pub fn disc(&mut self, cx: f32, cy: f32, r: f32, col: u32) {
        let (dx0, dx1) = self.span(cx - r, cx + r, self.ox, self.w);
        let (dy0, dy1) = self.span(cy - r, cy + r, self.oy, self.h);
        for yi in dy0..dy1 {
            let ly = ((yi as f32) + 0.5 - self.oy) / self.s;
            for xi in dx0..dx1 {
                let lx = ((xi as f32) + 0.5 - self.ox) / self.s;
                let (dx, dy) = (lx - cx, ly - cy);
                let a = clamp01(0.5 + (r * r - dx * dx - dy * dy) / (2.0 * r));
                if a > 0.0 {
                    let i = yi * self.w + xi;
                    self.px[i] = mix(self.px[i], col, a);
                }
            }
        }
    }

    fn panel(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.rr(x0 - 1.0, y0 - 1.0, x1 + 1.0, y1 + 1.0, 6.0, PANEL_EDGE, PANEL_EDGE);
        self.rr(x0, y0, x1, y1, 5.0, PANEL, PANEL);
    }

    /// Testo 3x5 con scala `sz` (px logici per punto del font).
    fn text(&mut self, s: &[u8], x: f32, y: f32, sz: f32, col: u32) {
        let mut cx = x;
        for &ch in s {
            for (row, bits) in glyph(ch).iter().enumerate() {
                for k in 0..3 {
                    if bits & (4 >> k) != 0 {
                        let gx = cx + k as f32 * sz;
                        let gy = y + row as f32 * sz;
                        self.rr(gx, gy, gx + sz, gy + sz, 0.0, col, col);
                    }
                }
            }
            cx += 4.0 * sz;
        }
    }

    fn text_c(&mut self, s: &[u8], y: f32, sz: f32, col: u32) {
        let w = s.len() as f32 * 4.0 * sz - sz;
        self.text(s, (PW as f32 - w) / 2.0, y, sz, col);
    }
}

/// Disegna una scena completa nel buffer `px` (`w` x `h`).
pub fn render(g: &Game, px: &mut [u32], w: usize, h: usize) {
    let mut c = Cv::new(px, w, h);
    c.bg();
    let pw = PW as f32;
    let ph = PH as f32;

    c.rr(-3.0, -3.0, pw + 3.0, ph + 3.0, 4.0, FRAME_T, FRAME_B);
    c.rr(0.0, 0.0, pw, ph, 2.0, FIELD_T, FIELD_B);

    for r in 0..ROWS {
        for col in 0..COLS {
            let k = g.cells[r][col];
            if k == EMPTY {
                continue;
            }
            let x = (BX0 + col as i32 * (BW + BGX)) as f32;
            let y = (BY0 + r as i32 * (BH + BGY)) as f32;
            let (t, b) = match k {
                NORMAL => (ROW[r], shade(ROW[r])),
                CRACKED => (CRACK_T, CRACK_B),
                _ => (STEEL_T, STEEL_B),
            };
            c.rr(x, y, x + BW as f32, y + BH as f32, 1.0, t, b);
            if k == CRACKED {
                c.rr(x + 6.0, y + 1.0, x + 8.0, y + 4.0, 0.5, CRACK_LINE, CRACK_LINE);
                c.rr(x + 14.0, y + 3.0, x + 17.0, y + 6.0, 0.5, CRACK_LINE, CRACK_LINE);
            }
        }
    }

    for d in g.drops.iter().flatten() {
        let (dx, dy) = (d.0 as f32 / SUB as f32, d.1 as f32 / SUB as f32);
        c.rr(dx - 3.0, dy - 1.5, dx + 3.0, dy + 1.5, 1.5, DROP_T, DROP_B);
    }

    let hw = g.half() as f32 / SUB as f32;
    let px_ = g.paddle as f32 / SUB as f32;
    c.rr(px_ - hw, PAD_Y as f32, px_ + hw, (PAD_Y + PAD_H) as f32, 1.5, PAD_T, PAD_B);

    let mut buf = [0u8; 10];
    let hud = num(g.score, &mut buf);
    let hud_len = hud.len();
    let _ = hud_len;
    c.text(b"LIVELLO", 4.0, 3.0, 1.0, TXT);
    let lvl = [b'0' + g.level as u8 + 1];
    c.text(&lvl, 36.0, 3.0, 1.0, TXT);

    if g.phase != Phase::Title {
        let (bx, by) = (g.ball.0 as f32 / SUB as f32, g.ball.1 as f32 / SUB as f32);
        c.disc(bx, by, BALL_R as f32 / SUB as f32, BALL);
    }

    for i in 0..g.lives as usize {
        let x = pw - 6.0 - i as f32 * 6.0;
        c.disc(x, 6.0, 1.5, BALL);
    }

    match g.phase {
        Phase::Title => {
            c.panel(30.0, 40.0, 210.0, 98.0);
            c.text_c(b"WILKANOID", 52.0, 3.0, TXT);
            c.text_c(b"PREMI SPAZIO", 80.0, 1.0, TXT);
        }
        Phase::Ready => c.text_c(b"PREMI SPAZIO", 96.0, 1.0, TXT),
        Phase::Lost => {
            c.panel(60.0, 50.0, 180.0, 80.0);
            c.text_c(b"VITA PERSA", 62.0, 2.0, TXT);
        }
        Phase::Over | Phase::Won => {
            c.panel(50.0, 40.0, 190.0, 98.0);
            let title: &[u8] = if g.phase == Phase::Over { b"GAME OVER" } else { b"VITTORIA" };
            c.text_c(title, 52.0, 2.0, TXT);
            c.text_c(b"PUNTI", 78.0, 1.0, TXT);
            let s = num(g.score, &mut buf);
            c.text_c(s, 88.0, 1.0, TXT);
        }
        _ => {}
    }
}
