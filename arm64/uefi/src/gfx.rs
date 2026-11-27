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
    pub fn put(&self, x: u32, y: u32, c: u32) {
        if x >= self.w || y >= self.h {
            return;
        }
        let v = self.layout.pack(c);
        let off = ((y * self.ppl + x) * self.layout.bytes) as usize;
        for i in 0..self.layout.bytes as usize {
            unsafe { self.base.add(off + i).write_volatile((v >> (8 * i)) as u8) };
        }
    }

    pub fn rect(&self, x: u32, y: u32, w: u32, h: u32, c: u32) {
        for yy in y..y.saturating_add(h).min(self.h) {
            for xx in x..x.saturating_add(w).min(self.w) {
                self.put(xx, yy, c);
            }
        }
    }
}

// Colori del tema 0 di HaikuOS (RRGGBB).
pub const BG: u32 = 0x08_10_31;
pub const BAR: u32 = 0x00_A5_FF;
pub const WINDOW: u32 = 0xFF_FF_FF;
pub const TITLE: u32 = 0xFF_84_00;
/// Colori delle nove icone (una per app), come in docs/desktop-concept.md.
pub const ICONS: [u32; 9] = [
    0xE8_A3_3D, 0x4F_C3_C1, 0xE0_7A_5F, 0x8E_7D_BE, 0xF2_CC_8F, 0x81_B2_9A, 0xEF_47_6F, 0x06_D6_A0, 0xFF_D1_66,
];

/// Desktop: sfondo, barra in alto, una finestra e una griglia 3x3 di icone (ogni icona è un quadrato pieno).
pub fn draw_desktop(f: &Fb) {
    f.rect(0, 0, f.w, f.h, BG);
    let bar = (f.h / 20).max(12);
    f.rect(0, 0, f.w, bar, BAR);
    f.rect(f.w / 4, f.h / 4, f.w / 2, f.h / 2, WINDOW);
    f.rect(f.w / 4, f.h / 4, f.w / 2, bar, TITLE);
    let cell = (f.w / 10).clamp(32, 96);
    let step = cell + cell / 2 + cell / 2;
    let margin = cell / 2;
    for (i, &c) in ICONS.iter().enumerate() {
        let (col, row) = ((i % 3) as u32, (i / 3) as u32);
        f.rect(margin + col * step, bar + margin + row * step, cell, cell, c);
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
        draw_desktop(&f);
        let cell = (w / 10).clamp(32, 96) as usize;
        for &c in ICONS.iter() {
            assert_eq!(conta(&buf, l, w, h, ppl, c), cell * cell, "icona {:06X}", c);
        }
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
