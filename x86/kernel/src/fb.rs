//! Framebuffer lineare RGB (da GRUB/Multiboot2 o da stage2), salvato a 0x600.
//! Formati: 15, 16, 24 e 32 bit con posizione e dimensione dei canali letti dal firmware.
//! Il sistema grafico lavora a ~640 colonne logiche: ogni pixel logico diventa scale×scale (bibbia §9).
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

/// Temi di colore: stessi quattro token (T0..T3) con valori diversi. Tema 0 è il riferimento (bibbia §3).
/// Da verificare la leggibilità dei temi 1 e 2.
pub const THEMES: [[u32; 4]; 3] = [
    [0x08_10_31, 0xFF_FF_FF, 0xFF_84_00, 0x00_A5_FF],
    [0x0B_2E_1C, 0xF2_F7_F2, 0x7F_D1_7F, 0x2E_8B_57],
    [0x1E_1E_1E, 0xEE_EE_EE, 0xFF_B0_00, 0x5A_5A_5A],
];

/// Buffer di lavoro in RAM (32 bit per pixel): ogni quadro si disegna qui e poi si copia tutto insieme.
const BACK: usize = 0x0400_0000;
/// Copia massima del buffer di lavoro: 3840×2160×4 byte (4K, ~32 MB).
const BACK_MAX: usize = 3840 * 2160 * 4;

pub struct Fb {
    base: usize,
    back: usize,
    pitch: usize,
    bpp: usize,
    pub w: i32,
    pub h: i32,
    /// Pixel fisico per pixel logico: circa 640 colonne logiche su qualsiasi schermo.
    pub scale: i32,
    /// Canali: [rpos, rsize, gpos, gsize, bpos, bsize] (come li scrive il loader).
    rgb: [u8; 6],
}

/// Legge la configurazione grafica; None se non c'è un modo RGB valido (si resta sul testo).
pub fn init() -> Option<Fb> {
    unsafe {
        let rd = |o: usize| core::ptr::read_volatile((INFO + o) as *const u32);
        let rb = |o: usize| core::ptr::read_volatile((INFO + o) as *const u8);
        if rd(0) != MAGIC || rb(0x22) != 1 {
            return None;
        }
        let w = rd(12) as i32;
        let h = rd(16) as i32;
        let (pitch, bpp) = (rd(20) as usize, rd(24) as usize);
        let rgb = [rb(0x1C), rb(0x1D), rb(0x1E), rb(0x1F), rb(0x20), rb(0x21)];
        // Canali validi (1..=8 bit, dentro il pixel) e riga almeno larga quanto i pixel.
        let ok_ch = |p: u8, s: u8| (1..=8).contains(&s) && p as usize + s as usize <= bpp;
        if w <= 0 || h <= 0 || (w as usize) * (h as usize) * 4 > BACK_MAX
            || !matches!(bpp, 15 | 16 | 24 | 32) || pitch < w as usize * ((bpp + 7) / 8)
            || !ok_ch(rgb[0], rgb[1]) || !ok_ch(rgb[2], rgb[3]) || !ok_ch(rgb[4], rgb[5])
        {
            return None;
        }
        Some(Fb {
            base: rd(8) as usize,
            back: BACK,
            w,
            h,
            pitch,
            bpp,
            scale: scale_for(w),
            rgb,
        })
    }
}

/// Converte un colore 0xRRGGBB nel formato del pixel (canali ridotti alla dimensione reale).
fn pack(rgb: &[u8; 6], c: u32) -> u32 {
    let ch = |v: u32, pos: u8, size: u8| ((v & 0xFF) >> (8 - size)) << pos;
    ch(c >> 16, rgb[0], rgb[1]) | ch(c >> 8, rgb[2], rgb[3]) | ch(c, rgb[4], rgb[5])
}

/// Scala intera: larghezza logica vicina a 640 (1024→2, 1280→2, 1920→3, 640→1).
pub fn scale_for(w: i32) -> i32 {
    ((w + 320) / 640).max(1)
}

impl Fb {
    /// Canvas logico: w/scale × h/scale, pixel fisici moltiplicati per scale.
    pub fn canvas(&self, theme: usize) -> Scaled<'_> {
        Scaled { fb: self, lw: self.w / self.scale, lh: self.h / self.scale, pal: &THEMES[theme] }
    }

    pub fn bpp(&self) -> usize {
        self.bpp
    }

    pub fn pitch(&self) -> usize {
        self.pitch
    }

    fn put(&self, x: i32, y: i32, c: u32) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let p = self.back + (y as usize * self.w as usize + x as usize) * 4;
        unsafe { core::ptr::write_volatile(p as *mut u32, c) }
    }

    /// Riempie un rettangolo fisico (ritagliato) nel buffer di lavoro, riga per riga.
    fn span(&self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        let x0 = x.max(0);
        let x1 = (x + w).min(self.w);
        if x0 >= x1 {
            return;
        }
        for py in y.max(0)..(y + h).min(self.h) {
            let p = (self.back + (py as usize * self.w as usize + x0 as usize) * 4) as *mut u32;
            // Buffer di lavoro in RAM (non MMIO): scrittura normale, LLVM la vettorizza.
            unsafe { core::slice::from_raw_parts_mut(p, (x1 - x0) as usize).fill(c) }
        }
    }

    /// Copia il buffer di lavoro sulla scheda video in un solo passo (niente sfarfallio).
    pub fn flush(&self) {
        let (w, h) = (self.w as usize, self.h as usize);
        let nb = (self.bpp + 7) / 8;
        // Percorso veloce: 32 bit con canali standard (B, G, R).
        let std32 = self.bpp == 32 && self.rgb == [16, 8, 8, 8, 0, 8];
        unsafe {
            for y in 0..h {
                let src = (self.back + y * w * 4) as *const u32;
                let dst = self.base + y * self.pitch;
                if std32 {
                    core::ptr::copy_nonoverlapping(src, dst as *mut u32, w);
                    continue;
                }
                for x in 0..w {
                    let v = pack(&self.rgb, core::ptr::read_volatile(src.add(x)));
                    let p = dst + x * nb;
                    for i in 0..nb {
                        core::ptr::write_volatile((p + i) as *mut u8, (v >> (8 * i)) as u8);
                    }
                }
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
        for dy in 0..self.fb.scale {
            for dx in 0..self.fb.scale {
                self.fb.put(x * self.fb.scale + dx, y * self.fb.scale + dy, c);
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
        let k = cell.w * self.fb.scale;
        let (ox, oy) = (cell.x * self.fb.scale, cell.y * self.fb.scale);
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
            self.fb.span(ox + x0, oy + y0, x1 - x0, y1 - y0, col(self.pal, t));
        }
    }

    /// Riempimento a span: ogni riga logica è una sola scrittura contigua nel buffer (vettorizzata da LLVM).
    fn fill(&mut self, r: Rect, t: Tok) {
        let c = col(self.pal, t);
        let s = self.fb.scale;
        let (x0, x1) = (r.x.max(0), (r.x + r.w).min(self.lw));
        if x0 >= x1 {
            return;
        }
        for y in r.y.max(0)..(r.y + r.h).min(self.lh) {
            self.fb.span(x0 * s, y * s, (x1 - x0) * s, s, c);
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
