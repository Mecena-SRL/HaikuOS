//! Framebuffer VBE lineare (impostato in stage2 a 640×480, 24 o 32 bit, salvato a 0x600).
//! Il sistema grafico lavora a 320×240 logici: ogni pixel logico diventa 2×2 (bibbia §9).
use haiku_gui::layout::Rect;
use haiku_gui::render::Canvas;
use haiku_gui::Tok;

// Il font 3×5 è quello del GBA (solo maiuscole, puro core).
#[path = "../../../gba/src/font.rs"]
mod font;

const INFO: usize = 0x600;
const MAGIC: u32 = 0x5642_4531;
pub const SCALE: i32 = 2;

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
    pub fn canvas(&self) -> Scaled<'_> {
        Scaled { fb: self, lw: self.w / SCALE, lh: self.h / SCALE }
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
}

impl<'a> Scaled<'a> {
    pub fn size(&self) -> (i32, i32) {
        (self.lw, self.lh)
    }

    fn block(&self, x: i32, y: i32, c: u32) {
        for dy in 0..SCALE {
            for dx in 0..SCALE {
                self.fb.put(x * SCALE + dx, y * SCALE + dy, c);
            }
        }
    }
}

impl<'a> Canvas for Scaled<'a> {
    fn fill(&mut self, r: Rect, t: Tok) {
        let c = t.rgb();
        for y in r.y.max(0)..(r.y + r.h).min(self.lh) {
            for x in r.x.max(0)..(r.x + r.w).min(self.lw) {
                self.block(x, y, c);
            }
        }
    }

    fn text(&mut self, x: i32, y: i32, s: &str, t: Tok) {
        let c = t.rgb();
        for (i, &ch) in s.as_bytes().iter().enumerate() {
            let g = font::glyph(ch);
            for (row, bits) in g.iter().enumerate() {
                for col in 0..font::GLYPH_W {
                    if bits & (0b100 >> col) != 0 {
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
