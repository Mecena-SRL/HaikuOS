//! Grafica GOP: formato pixel generico (8 bit per canale o maschere, 16/24/32 bit) e desktop.
//! Solo `core`: i test girano anche sull'host (vedi scripts/test-host.sh).

/// Posizione e dimensione dei canali R, G, B nel pixel, e byte per pixel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub shift: [u32; 3],
    pub size: [u32; 3],
    pub bytes: u32,
}

impl Layout {
    /// PixelRedGreenBlueReserved8BitPerColor: R al byte 0.
    pub const RGB8: Layout = Layout { shift: [0, 8, 16], size: [8, 8, 8], bytes: 4 };
    /// PixelBlueGreenRedReserved8BitPerColor: B al byte 0.
    pub const BGR8: Layout = Layout { shift: [16, 8, 0], size: [8, 8, 8], bytes: 4 };

    /// PixelBitMask: posizione e dimensione dalle maschere. Se nessun bit supera il 16°, il pixel è di 2 byte.
    pub fn from_masks(r: u32, g: u32, b: u32) -> Layout {
        let bytes = if (r | g | b) <= 0xFFFF { 2 } else { 4 };
        Layout {
            shift: [r.trailing_zeros(), g.trailing_zeros(), b.trailing_zeros()],
            size: [r.count_ones(), g.count_ones(), b.count_ones()],
            bytes,
        }
    }

    /// Converte 0xRRGGBB (8 bit per canale) nel valore del pixel, scalando ogni canale alla sua dimensione.
    pub fn pack(&self, c: u32) -> u32 {
        let v = [(c >> 16) & 0xFF, (c >> 8) & 0xFF, c & 0xFF];
        let mut out = 0u32;
        for i in 0..3 {
            let n = self.size[i];
            if n == 0 {
                continue;
            }
            let max = (1u64 << n) - 1;
            let q = (v[i] as u64 * max + 127) / 255;
            out |= (q as u32) << self.shift[i];
        }
        out
    }
}

#[path = "../../../gba/src/font.rs"]
mod font;

/// Superficie di disegno sul framebuffer GOP. `base` deve puntare a una memoria valida di `ppl * h * bytes`.
pub struct Fb {
    pub base: *mut u8,
    pub w: u32,
    pub h: u32,
    /// Pixel per riga (può essere maggiore di `w`).
    pub ppl: u32,
    pub layout: Layout,
}

impl Fb {
    /// Riempie un rettangolo (ritagliato allo schermo). Il colore si converte una volta sola.
    pub fn rect(&self, x: u32, y: u32, w: u32, h: u32, c: u32) {
        let v = self.layout.pack(c);
        let n = self.layout.bytes as usize;
        for yy in y..y.saturating_add(h).min(self.h) {
            let row = (yy * self.ppl) as usize * n;
            for xx in x..x.saturating_add(w).min(self.w) {
                let off = row + xx as usize * n;
                for i in 0..n {
                    unsafe { self.base.add(off + i).write_volatile((v >> (8 * i)) as u8) };
                }
            }
        }
    }
}

impl Fb {
    /// Testo maiuscolo con font 3x5 ingrandito di `scale` volte. Il bit più a sinistra è la colonna 0.
    pub fn text(&self, x: u32, y: u32, s: &str, scale: u32, c: u32) {
        let mut cx = x;
        for b in s.bytes() {
            for (ry, row) in font::glyph(b).iter().enumerate() {
                for col in 0..font::GLYPH_W as u32 {
                    if row & (0b100 >> col) != 0 {
                        self.rect(cx + col * scale, y + ry as u32 * scale, scale, scale, c);
                    }
                }
            }
            cx += font::ADVANCE as u32 * scale;
        }
    }
}

/// Larghezza in pixel di `s` con il font a `scale` (senza lo spazio finale).
pub fn text_width(s: &str, scale: u32) -> u32 {
    if s.is_empty() {
        0
    } else {
        (s.len() as u32 * font::ADVANCE as u32 - 1) * scale
    }
}

/// Nomi delle nove app, nell'ordine delle icone (come nel desktop x86).
pub const APPS: [&str; 9] = ["FILE", "TERM", "CALC", "WEB", "EDIT", "SET", "ORA", "READ", "DOC"];

/// Temi (RRGGBB): [sfondo, testo e finestra, accento del titolo, barra]. Come in x86/kernel/src/fb.rs.
pub const THEMES: [[u32; 4]; 3] = [
    [0x08_10_31, 0xFF_FF_FF, 0xFF_84_00, 0x00_A5_FF],
    [0x0B_2E_1C, 0xF2_F7_F2, 0x7F_D1_7F, 0x2E_8B_57],
    [0x1E_1E_1E, 0xEE_EE_EE, 0xFF_B0_00, 0x5A_5A_5A],
];
/// Indice dell'icona SET (impostazioni, cambio tema).
pub const SET_INDEX: usize = 5;
/// Colori delle nove icone (una per app), come in docs/desktop-concept.md.
pub const ICONS: [u32; 9] = [
    0xE8_A3_3D, 0x4F_C3_C1, 0xE0_7A_5F, 0x8E_7D_BE, 0xF2_CC_8F, 0x81_B2_9A, 0xEF_47_6F, 0x06_D6_A0, 0xFF_D1_66,
];

/// Primi `n` caratteri di `s`.
fn fit(s: &str, n: usize) -> &str {
    &s[..s.len().min(n)]
}

/// Ultimi `n` caratteri di `s` (per il prompt, che deve restare visibile mentre si scrive).
fn tail(s: &str, n: usize) -> &str {
    &s[s.len().saturating_sub(n)..]
}

/// Stato visibile del desktop: icona selezionata e app aperta (la finestra mostra il suo nome).
#[derive(Clone, Copy, Default)]
pub struct View {
    pub sel: Option<usize>,
    pub open: Option<usize>,
}

/// Desktop: sfondo, barra, finestra a destra della griglia, 9 icone con etichette.
/// Indici delle icone con contenuto: TERM e CALC.
pub const TERM_INDEX: usize = 1;
pub const CALC_INDEX: usize = 2;
/// Indice dell'icona ORA (orologio).
pub const CLOCK_INDEX: usize = 6;

/// Testo delle app con contenuto: `body[0]` e `body[1]` sono due righe (CALC: display; TERM: output e prompt).
pub fn draw_desktop(f: &Fb, v: View, body: [&str; 2], theme: usize) {
    let [bg, window, title, bar_c] = THEMES[theme % THEMES.len()];
    f.rect(0, 0, f.w, f.h, bg);
    let bar = (f.h / 20).max(12);
    f.rect(0, 0, f.w, bar, bar_c);
    let ts = (bar / 6).max(1);
    f.text(8, (bar - 5 * ts) / 2, "HAIKUOS", ts, bg);
    // Finestra a destra della griglia di icone, così non copre le etichette.
    let (wx, wy, ww, wh) = (f.w * 3 / 5, f.h / 4, f.w / 3, f.h / 2);
    f.rect(wx, wy, ww, wh, window);
    f.rect(wx, wy, ww, bar, title);
    match v.open {
        None => f.text(wx + 8, wy + (bar - 5 * ts) / 2, "BENVENUTO", ts, bg),
        Some(i) => {
            f.text(wx + 8, wy + (bar - 5 * ts) / 2, "APERTA", ts, bg);
            if i == CALC_INDEX || i == TERM_INDEX {
                let n = ((ww - 16) / (4 * ts)) as usize;
                let (a, b) = (fit(body[0], n), tail(body[1], n));
                f.text(wx + 8, wy + bar + 8, a, ts, bg);
                f.text(wx + 8, wy + bar + 8 + 6 * ts, b, ts, bg);
            } else if i == SET_INDEX {
                f.text(wx + 8, wy + bar + 8, body[0], ts * 2, bg);
                f.text(wx + 8, wy + bar + 8 + 14 * ts, body[1], ts, bg);
            } else if i == CLOCK_INDEX {
                let sc = ((ww - 16) / 32).min(ts * 3).max(1);
                f.text(wx + 8, wy + bar + 8, body[0], sc, bg);
            } else {
                f.text(wx + 8, wy + bar + 8, APPS[i], ts * 2, bg);
            }
        }
    }
    let cell = (f.w / 10).clamp(32, 96);
    let step = cell + cell / 2 + cell / 2;
    let margin = cell / 2;
    let ls = (cell / 16).max(1);
    for (i, &c) in ICONS.iter().enumerate() {
        let (col, row) = ((i % 3) as u32, (i / 3) as u32);
        let (x, y) = (margin + col * step, bar + margin + row * step);
        f.rect(x, y, cell, cell, c);
        let label = APPS[i];
        let lx = x + cell / 2 - text_width(label, ls) / 2;
        f.text(lx, y + cell + ls * 2, label, ls, window);
        if v.sel == Some(i) {
            // Cornice fuori dall'icona: 4 px bianchi attorno al quadrato.
            let b = 4;
            f.rect(x - b, y - b, cell + 2 * b, b, window);
            f.rect(x - b, y + cell, cell + 2 * b, b, window);
            f.rect(x - b, y, b, cell, window);
            f.rect(x + cell, y, b, cell, window);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb8_e_bgr8() {
        assert_eq!(Layout::RGB8.pack(0x112233), 0x0033_2211);
        assert_eq!(Layout::BGR8.pack(0x112233), 0x0011_2233);
    }

    #[test]
    fn rgb565_da_maschere() {
        let l = Layout::from_masks(0xF800, 0x07E0, 0x001F);
        assert_eq!(l.bytes, 2);
        assert_eq!(l.pack(0xFF0000), 0xF800);
        assert_eq!(l.pack(0x00FF00), 0x07E0);
        assert_eq!(l.pack(0x0000FF), 0x001F);
        assert_eq!(l.pack(0xFFFFFF), 0xFFFF);
    }

    fn conta(buf: &[u8], l: Layout, w: u32, h: u32, ppl: u32, c: u32) -> usize {
        let mut n = 0;
        for y in 0..h {
            for x in 0..w {
                let off = ((y * ppl + x) * l.bytes) as usize;
                let mut v = 0u32;
                for i in 0..l.bytes as usize {
                    v |= (buf[off + i] as u32) << (8 * i);
                }
                if v == l.pack(c) {
                    n += 1;
                }
            }
        }
        n
    }

    fn prova(l: Layout, w: u32, h: u32, ppl: u32) {
        let mut buf = vec![0u8; (ppl * h * l.bytes) as usize];
        let f = Fb { base: buf.as_mut_ptr(), w, h, ppl, layout: l };
        draw_desktop(&f, View::default(), ["", ""], 0);
        let cell = (w / 10).clamp(32, 96) as usize;
        for &c in ICONS.iter() {
            assert_eq!(conta(&buf, l, w, h, ppl, c), cell * cell, "icona {:06X}", c);
        }
    }

    #[test]
    fn testo_disegnato_con_font_3x5() {
        let l = Layout::BGR8;
        let (w, h, ppl) = (64u32, 16u32, 64u32);
        let mut buf = vec![0u8; (ppl * h * l.bytes) as usize];
        let f = Fb { base: buf.as_mut_ptr(), w, h, ppl, layout: l };
        f.text(0, 0, "T", 2, 0xFFFFFF);
        // Il glifo di T a scala 2: ogni pixel acceso diventa 2x2.
        let accesi: u32 = font::glyph(b'T').iter().map(|r| r.count_ones()).sum();
        assert_eq!(conta(&buf, l, w, h, ppl, 0xFFFFFF) as u32, accesi * 4);
        assert_eq!(text_width("AB", 2), (2 * 4 - 1) * 2);
    }

    #[test]
    fn desktop_32_bit_bgr() {
        prova(Layout::BGR8, 800, 600, 800);
    }

    #[test]
    fn desktop_16_bit_rgb565_con_riga_piu_larga() {
        prova(Layout::from_masks(0xF800, 0x07E0, 0x001F), 800, 600, 832);
    }
}
