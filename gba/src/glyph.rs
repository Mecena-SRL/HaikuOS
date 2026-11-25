//! Font 3x5 (lettere maiuscole, cifre, spazio) e disegno di testo in un buffer di pixel.
//! Ogni glifo è 5 righe da 3 bit, bit più alto = colonna sinistra.

pub const GW: usize = 3;
pub const GH: usize = 5;
/// Avanzamento tra un carattere e il successivo (glifo + 1 colonna vuota).
pub const ADVANCE: usize = GW + 1;

/// Ordine dei glifi: `CHARS[i]` ha righe `FONT[i]`.
pub const CHARS: &[u8] = b" ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
pub const FONT: [[u8; GH]; 37] = [
    [0b000, 0b000, 0b000, 0b000, 0b000], // spazio
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
    [0b110, 0b101, 0b101, 0b101, 0b101], // N
    [0b010, 0b101, 0b101, 0b101, 0b010], // O
    [0b110, 0b101, 0b110, 0b100, 0b100], // P
    [0b010, 0b101, 0b101, 0b110, 0b011], // Q
    [0b110, 0b101, 0b110, 0b101, 0b101], // R
    [0b011, 0b100, 0b010, 0b001, 0b110], // S
    [0b111, 0b010, 0b010, 0b010, 0b010], // T
    [0b101, 0b101, 0b101, 0b101, 0b111], // U
    [0b101, 0b101, 0b101, 0b101, 0b010], // V
    [0b101, 0b101, 0b111, 0b111, 0b101], // W
    [0b101, 0b101, 0b010, 0b101, 0b101], // X
    [0b101, 0b101, 0b010, 0b010, 0b010], // Y
    [0b111, 0b001, 0b010, 0b100, 0b111], // Z
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b110, 0b001, 0b010, 0b100, 0b111], // 2
    [0b110, 0b001, 0b010, 0b001, 0b110], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b110, 0b001, 0b110], // 5
    [0b011, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b010, 0b010, 0b010], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b110], // 9
];

/// Righe del glifo per `c` (minuscole convertite in maiuscole). Carattere sconosciuto: spazio.
pub fn glyph(c: u8) -> [u8; GH] {
    let c = c.to_ascii_uppercase();
    match CHARS.iter().position(|&k| k == c) {
        Some(i) => FONT[i],
        None => FONT[0],
    }
}

/// Disegna `text` nel buffer `px` largo `w` e alto `h`, partendo da (x, y). Ritaglia ai bordi.
pub fn draw_text(px: &mut [u16], w: usize, h: usize, x: usize, y: usize, text: &[u8], color: u16) {
    for (i, &c) in text.iter().enumerate() {
        let gx = x + i * ADVANCE;
        let rows = glyph(c);
        for (ry, bits) in rows.iter().enumerate() {
            for col in 0..GW {
                if bits & (0b100 >> col) == 0 {
                    continue;
                }
                let (px_x, px_y) = (gx + col, y + ry);
                if px_x < w && px_y < h {
                    px[px_y * w + px_x] = color;
                }
            }
        }
    }
}

/// Colonne e righe visibili del testo a capo (area 64x48, origine (2, 2), riga di 6 px).
pub const COLS: usize = 15;
pub const ROWS: usize = 6;
pub const ORIGIN: usize = 2;
pub const LINE: usize = GH + 1;

/// Disegna `text` a capo ogni `COLS` caratteri, mostrando le ultime `ROWS` righe.
/// Il carattere in attesa `pending` va in fondo, alla posizione successiva all'ultimo.
pub fn draw_wrapped(px: &mut [u16], w: usize, h: usize, text: &[u8], pending: u8, text_color: u16, pending_color: u16) {
    let len = text.len();
    let total_lines = len / COLS + 1;
    let start = total_lines.saturating_sub(ROWS);
    for i in 0..=len {
        let c = if i < len { text[i] } else { pending };
        let color = if i < len { text_color } else { pending_color };
        let line = i / COLS;
        if line < start {
            continue;
        }
        let row = line - start;
        let x = ORIGIN + (i % COLS) * ADVANCE;
        let y = ORIGIN + row * LINE;
        draw_text(px, w, h, x, y, &[c], color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lettera_l_e_cifra_1() {
        assert_eq!(glyph(b'L'), [0b100, 0b100, 0b100, 0b100, 0b111]);
        assert_eq!(glyph(b'1'), [0b010, 0b110, 0b010, 0b010, 0b111]);
    }

    #[test]
    fn minuscole_e_sconosciuti() {
        assert_eq!(glyph(b'h'), glyph(b'H'));
        assert_eq!(glyph(b'?'), FONT[0]);
    }

    #[test]
    fn a_capo_e_scorrimento() {
        // 15 caratteri: riga piena. Il cursore (pending) va alla riga successiva, colonna 0.
        let mut px = std::vec![0u16; 64 * 48];
        draw_wrapped(&mut px, 64, 48, &[b'I'; 15], b'X', 1, 2);
        assert_eq!(px[ORIGIN * 64 + ORIGIN], 1);
        assert_eq!(px[(ORIGIN + LINE) * 64 + ORIGIN], 2);
        // 120 caratteri: 9 righe con il cursore. Scorrono via le prime 3, restano 6 righe visibili.
        let mut px = std::vec![0u16; 64 * 48];
        draw_wrapped(&mut px, 64, 48, &[b'I'; COLS * 8], b'X', 1, 2);
        assert_eq!(px[(ORIGIN + (ROWS - 1) * LINE) * 64 + ORIGIN], 2); // cursore in fondo
        let visibili = px.iter().filter(|&&p| p == 1).count();
        assert_eq!(visibili, 5 * COLS * 9); // 5 righe piene di 'I' (9 pixel per glifo)
    }

    #[test]
    fn disegna_e_ritaglia() {
        let mut px = std::vec![0u16; 8 * 6];
        draw_text(&mut px, 8, 6, 0, 0, b"I", 7);
        // "I": riga alta 111, poi 010 x3, poi 111.
        assert_eq!(px[0], 7);
        assert_eq!(px[1], 7);
        assert_eq!(px[2], 7);
        assert_eq!(px[8 + 1], 7);
        assert_eq!(px[8], 0);
        // Ritaglio: testo che esce dal bordo non scrive fuori dal buffer.
        draw_text(&mut px, 8, 6, 6, 4, b"II", 9);
        assert_eq!(px.len(), 48);
    }
}
