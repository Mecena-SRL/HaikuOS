//! Scala intera per schermi grandi (bibbia grafica §9): il disegno 240×160 si
//! ingrandisce di un fattore intero, senza interpolazione. Le bande extra sono `bg`.
//! Nessuna dipendenza da hardware: il test host controlla i byte prodotti.

pub const LW: usize = 240;
pub const LH: usize = 160;

/// Fattore intero più grande che entra nello schermo (minimo 1).
pub const fn factor(w: usize, h: usize) -> usize {
    let a = w / LW;
    let b = h / LH;
    let k = if a < b { a } else { b };
    if k == 0 {
        1
    } else {
        k
    }
}

/// Copia `src` (LW×LH) in `dst` (w×h, riga = w pixel) con fattore `factor(w, h)`, centrato.
/// Ritorna false se le misure non corrispondono o lo schermo è più piccolo di LW×LH.
pub fn upscale(src: &[u16], dst: &mut [u16], w: usize, h: usize, bg: u16) -> bool {
    let k = factor(w, h);
    let (ow, oh) = (LW * k, LH * k);
    if src.len() != LW * LH || dst.len() != w * h || ow > w || oh > h {
        return false;
    }
    let (x0, y0) = ((w - ow) / 2, (h - oh) / 2);
    for p in dst.iter_mut() {
        *p = bg;
    }
    for sy in 0..LH {
        for dy in 0..k {
            let row = (y0 + sy * k + dy) * w + x0;
            for sx in 0..LW {
                let c = src[sy * LW + sx];
                for dx in 0..k {
                    dst[row + sx * k + dx] = c;
                }
            }
        }
    }
    true
}
