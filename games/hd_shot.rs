//! Screenshot ad alta risoluzione (solo host, std). Condiviso dai giochi via `#[path]`.
//! Il fotogramma logico resta quello del gioco: qui viene ingrandito di un intero
//! (vicino più prossimo, nitido per la pixel art) e centrato su fondo nero in `ow`×`oh`.

use std::io::{self, Write};

/// Scrive un PPM P6 `ow`×`oh` con il fotogramma `px` (`w`×`h`) ingrandito al massimo intero che entra.
pub fn write_fit(out: &mut impl Write, w: usize, h: usize, px: &[[u8; 3]], ow: usize, oh: usize) -> io::Result<()> {
    let s = (ow / w).min(oh / h).max(1);
    let (dw, dh) = (w * s, h * s);
    let (x0, y0) = ((ow - dw) / 2, (oh - dh) / 2);
    write!(out, "P6\n{ow} {oh}\n255\n")?;
    let mut row = vec![[0u8; 3]; ow];
    for y in 0..oh {
        row.fill([0; 3]);
        if (y0..y0 + dh).contains(&y) {
            let src = &px[(y - y0) / s * w..][..w];
            for x in 0..dw {
                row[x0 + x] = src[x / s];
            }
        }
        for p in &row {
            out.write_all(p)?;
        }
    }
    Ok(())
}
