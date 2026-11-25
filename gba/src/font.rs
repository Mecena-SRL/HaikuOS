//! Font 3x5 maiuscolo per le etichette del desktop (scelta provvisoria: più piccolo e riscrivibile).
//! Ogni riga è 3 bit, bit più a sinistra = colonna 0. Caratteri sconosciuti = spazio.
#![allow(dead_code)]

pub const GLYPH_W: usize = 3;
pub const GLYPH_H: usize = 5;
/// Avanzamento tra un carattere e il successivo (3 px + 1 di spazio).
pub const ADVANCE: usize = 4;

/// Righe del glifo per il carattere `c` (maiuscolo). Spazio per i caratteri non previsti.
pub fn glyph(c: u8) -> [u8; GLYPH_H] {
    match c.to_ascii_uppercase() {
        b':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        b'3' => [0b110, 0b001, 0b010, 0b001, 0b110],
        b'4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        b'5' => [0b111, 0b100, 0b110, 0b001, 0b110],
        b'6' => [0b011, 0b100, 0b110, 0b101, 0b010],
        b'7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        b'8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        b'9' => [0b111, 0b101, 0b111, 0b001, 0b110],
        b'J' => [0b001, 0b001, 0b001, 0b101, 0b010],
        b'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        b'Q' => [0b010, 0b101, 0b101, 0b110, 0b011],
        b'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        b'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        b'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        b'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        b'0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        b'1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        b'2' => [0b110, 0b001, 0b010, 0b100, 0b111],
        b'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        b'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        b'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        b'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        b'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        b'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        b'G' => [0b011, 0b100, 0b101, 0b101, 0b011],
        b'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        b'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        b'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        b'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        b'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        b'N' => [0b101, 0b111, 0b111, 0b111, 0b101],
        b'O' => [0b010, 0b101, 0b101, 0b101, 0b010],
        b'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        b'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        b'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        b'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        b'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        b'+' => [0b000, 0b010, 0b111, 0b010, 0b000],
        b'-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        b'*' => [0b000, 0b101, 0b010, 0b101, 0b000],
        b'/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        b'=' => [0b000, 0b111, 0b000, 0b111, 0b000],
        _ => [0; GLYPH_H],
    }
}

/// Larghezza in pixel di una stringa.
pub fn text_width(s: &[u8]) -> usize {
    if s.is_empty() {
        0
    } else {
        (s.len() - 1) * ADVANCE + GLYPH_W
    }
}

/// Disegna `s` in `fb` (larghezza `fb_w`, altezza `fb_h`) a (x, y), colore `c`. Fuori schermo = tagliato.
pub fn draw(fb: &mut [u16], fb_w: usize, fb_h: usize, x: usize, y: usize, s: &[u8], c: u16) {
    for (i, &ch) in s.iter().enumerate() {
        let g = glyph(ch);
        let gx = x + i * ADVANCE;
        for (row, bits) in g.iter().enumerate() {
            for col in 0..GLYPH_W {
                if bits & (0b100 >> col) != 0 {
                    let px = gx + col;
                    let py = y + row;
                    if px < fb_w && py < fb_h {
                        fb[py * fb_w + px] = c;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_label_glyph_has_pixels() {
        for &c in b"HAIKUOSFILESTERMGIOCHINTDCB012+-*/=" {
            assert!(glyph(c).iter().any(|&r| r != 0), "glifo vuoto per {}", c as char);
        }
    }

    #[test]
    fn unknown_char_is_blank_and_lowercase_maps_to_upper() {
        assert_eq!(glyph(b'?'), [0; GLYPH_H]);
        assert_eq!(glyph(b'a'), glyph(b'A'));
    }

    #[test]
    fn draw_writes_inside_and_clips() {
        let mut fb = [0u16; 8 * 8];
        draw(&mut fb, 8, 8, 6, 6, b"HI", 7);
        // Le colonne fuori dallo schermo vengono tagliate senza panico.
        assert!(fb.iter().any(|&p| p == 7));
        assert_eq!(text_width(b"HI"), ADVANCE + GLYPH_W);
        assert_eq!(text_width(b""), 0);
    }
}
