//! Framebuffer VBE lineare (impostato in stage2 a 640×480, 24 o 32 bit, salvato a 0x600).
//! Il sistema grafico lavora a 320×240 logici: ogni pixel logico diventa 2×2 (bibbia §9).
use haiku_gui::layout::Rect;
use haiku_gui::render::Canvas;
use haiku_gui::icons;
use haiku_gui::palette::ACCENTS;
use haiku_gui::Tok;

// Il font 3×5 è quello del GBA (solo maiuscole, puro core).
#[path = "../../../gba/src/font.rs"]
mod font;

const INFO: usize = 0x600;
const MAGIC: u32 = 0x5642_4531;
pub const SCALE: i32 = 2;

/// Temi di colore: stessi quattro token (T0..T3) con valori diversi. Tema 0 è il riferimento (bibbia §3).
/// Da verificare la leggibilità dei temi 1 e 2.
pub const THEMES: [[u32; 4]; 3] = [
    [0x08_10_31, 0xFF_FF_FF, 0xFF_84_00, 0x00_A5_FF],
    [0x0B_2E_1C, 0xF2_F7_F2, 0x7F_D1_7F, 0x2E_8B_57],
    [0x1E_1E_1E, 0xEE_EE_EE, 0xFF_B0_00, 0x5A_5A_5A],
];

pub struct Fb {
    base: usize,
    pitch: usize,
    bpp: usize,
    pub w: i32,
    pub h: i32,
}

/// Legge la configurazione VBE; None se stage2 non ha impostato un modo grafico.
pub fn init() -> Option<Fb> {
    unsafe {
        let rd = |o: usize| core::ptr::read_volatile((INFO + o) as *const u32);
        if rd(0) != MAGIC {
            return None;
        }
        Some(Fb {
            base: rd(8) as usize,
            w: rd(12) as i32,
            h: rd(16) as i32,
            pitch: rd(20) as usize,
            bpp: rd(24) as usize,
        })
    }
}

impl Fb {
    /// Canvas logico: 320×240 (w/2 × h/2) con scala intera.
    pub fn canvas(&self, theme: usize) -> Scaled<'_> {
        Scaled { fb: self, lw: self.w / SCALE, lh: self.h / SCALE, pal: &THEMES[theme] }
    }

    fn put(&self, x: i32, y: i32, c: u32) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let p = self.base + y as usize * self.pitch + x as usize * (self.bpp / 8);
        unsafe {
            if self.bpp == 32 {
                core::ptr::write_volatile(p as *mut u32, c);
            } else {
                // 24 bit: byte in ordine B, G, R
                core::ptr::write_volatile(p as *mut u8, c as u8);
                core::ptr::write_volatile((p + 1) as *mut u8, (c >> 8) as u8);
                core::ptr::write_volatile((p + 2) as *mut u8, (c >> 16) as u8);
            }
        }
    }
}

pub struct Scaled<'a> {
    fb: &'a Fb,
    lw: i32,
    lh: i32,
    pal: &'a [u32; 4],
}

/// Colore di un token: T0..T3 dal tema, `A(k)` accento fisso a 32 bit.
fn col(pal: &[u32; 4], t: Tok) -> u32 {
    match t {
        Tok::T0 => pal[0],
        Tok::T1 => pal[1],
        Tok::T2 => pal[2],
        Tok::T3 => pal[3],
        Tok::A(k) => ACCENTS[k as usize % ACCENTS.len()],
    }
}

/// Angolo smussato: un pixel del piatto (lato k, raggio r) è dentro se non cade fuori dal cerchio d'angolo.
fn inside_plate(px: i32, py: i32, k: i32, r: i32) -> bool {
    let d = |v: i32| if v < r { r - 1 - v } else if v >= k - r { v - (k - r) } else { 0 };
    let (dx, dy) = (d(px), d(py));
    dx * dx + dy * dy < r * r
}

impl<'a> Scaled<'a> {
    fn block(&self, x: i32, y: i32, c: u32) {
        for dy in 0..SCALE {
            for dx in 0..SCALE {
                self.fb.put(x * SCALE + dx, y * SCALE + dy, c);
            }
        }
    }

    fn rect_px(&self, x0: i32, y0: i32, x1: i32, y1: i32, c: u32) {
        for y in y0..y1 {
            for x in x0..x1 {
                self.fb.put(x, y, c);
            }
        }
    }
}

impl<'a> Canvas for Scaled<'a> {
    fn rounded(&self) -> bool {
        true
    }

    fn rich(&self) -> bool {
        true
    }

    /// Icona vera: piatto colorato con angoli smussati, poi i segni dell'icona (32 px per cella 16 logica).
    /// Con cella 8×8 (titoli) la scala è 16 px. Coordinate dei segni: griglia 32, scalate su `k`.
    fn icon(&mut self, cell: Rect, label: &str, app: u8) {
        let k = cell.w * SCALE;
        let (ox, oy) = (cell.x * SCALE, cell.y * SCALE);
        let r = k / 8;
        let plate = col(self.pal, Tok::A(app));
        for py in 0..k {
            for px in 0..k {
                if inside_plate(px, py, k, r) {
                    self.fb.put(ox + px, oy + py, plate);
                }
            }
        }
        for &(x, y, w, h, t) in icons::art(label) {
            let (x0, x1) = (x * k / 32, ((x + w) * k / 32).max(x * k / 32 + 1));
            let (y0, y1) = (y * k / 32, ((y + h) * k / 32).max(y * k / 32 + 1));
            self.rect_px(ox + x0, oy + y0, ox + x1, oy + y1, col(self.pal, t));
        }
    }

    fn fill(&mut self, r: Rect, t: Tok) {
        let c = col(self.pal, t);
        for y in r.y.max(0)..(r.y + r.h).min(self.lh) {
            for x in r.x.max(0)..(r.x + r.w).min(self.lw) {
                self.block(x, y, c);
            }
        }
    }

    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok) {
        let c = col(self.pal, t);
        for (i, &ch) in s.as_bytes().iter().enumerate() {
            let g = font::glyph(ch);
            for (row, bits) in g.iter().enumerate() {
                for col in 0..font::GLYPH_W {
                    if bits & (1 << (font::GLYPH_W - 1 - col)) != 0 {
                        let px = x + (i * font::ADVANCE + col) as i32;
                        let py = y + row as i32;
                        if px >= 0 && py >= 0 && px < self.lw && py < self.lh {
                            self.block(px, py, c);
                        }
                    }
                }
            }
        }
    }
}
