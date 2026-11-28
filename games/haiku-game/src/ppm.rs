//! PPM P6 (solo host).

use std::io::{self, Write};

/// Scrive il frame RGB `w`×`h` ingrandito di `scale` (vicino più prossimo).
pub fn write_scaled(out: &mut impl Write, w: usize, h: usize, px: &[[u8; 3]], scale: usize) -> io::Result<()> {
    let (ow, oh) = (w * scale, h * scale);
    write!(out, "P6\n{ow} {oh}\n255\n")?;
    for y in 0..oh {
        for x in 0..ow {
            out.write_all(&px[(y / scale) * w + x / scale])?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_and_scaling() {
        let px = [[1, 2, 3], [4, 5, 6]];
        let mut v = Vec::new();
        write_scaled(&mut v, 2, 1, &px, 2).unwrap();
        assert!(v.starts_with(b"P6\n4 2\n255\n"));
        assert_eq!(&v[11..], &[1, 2, 3, 1, 2, 3, 4, 5, 6, 4, 5, 6, 1, 2, 3, 1, 2, 3, 4, 5, 6, 4, 5, 6]);
    }
}
