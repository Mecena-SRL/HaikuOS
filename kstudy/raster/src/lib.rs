//! Vector coverage rasterizer: a closed polygon in 24.8 fixed point becomes
//! per-pixel anti-aliased coverage (u8, 0..=255), one scanline at a time.
//!
//! Method: each edge adds its signed area to a per-row accumulator, split at
//! pixel columns. A pixel's coverage is the running sum of that accumulator.
//! No floats, no heap; capacity is fixed by the const generics `W` (row width)
//! and `E` (max non-horizontal edges).

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

use core::ops::Range;

/// Point in 24.8 fixed point: 256 units per pixel. Pixel `i` covers `[i, i+1)`.
pub type Pt = (i32, i32);

const ONE: i32 = 256;
const FULL: i64 = (ONE * ONE) as i64; // area of one pixel, in 1/65536 px^2

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The polygon has more non-horizontal edges than the capacity `E`.
    Capacity,
}

#[derive(Clone, Copy)]
struct Edge {
    x0: i32,
    y0: i32, // top endpoint
    x1: i32,
    y1: i32, // bottom endpoint, y1 > y0
    dir: i32, // +1 if the polygon goes down across this edge, -1 if up
}

impl Edge {
    const ZERO: Edge = Edge { x0: 0, y0: 0, x1: 0, y1: 0, dir: 0 };

    /// x at height `y` (clamped to the edge's span by the caller).
    fn x_at(&self, y: i32) -> i32 {
        let num = (y - self.y0) as i64 * (self.x1 - self.x0) as i64;
        (self.x0 as i64 + num / (self.y1 - self.y0) as i64) as i32
    }
}

/// Rasterizer with fixed buffers: `W` output columns, `E` edges at most.
pub struct Rasterizer<const W: usize, const E: usize> {
    edges: [Edge; E],
    n: usize,
    acc: [i32; W],
    row: [u8; W],
}

impl<const W: usize, const E: usize> Default for Rasterizer<W, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const W: usize, const E: usize> Rasterizer<W, E> {
    pub const fn new() -> Self {
        Rasterizer { edges: [Edge::ZERO; E], n: 0, acc: [0; W], row: [0; W] }
    }

    /// Renders the closed polygon `pts` (implicitly closed last-to-first) for
    /// every row in `rows`. Columns are `0..W`. For each row, `emit(y, &cov)`
    /// is called with the coverage of that row. Nonzero winding, clamped.
    /// Returns `Err(Capacity)` if the polygon has more than `E` non-horizontal edges.
    pub fn render<F: FnMut(i32, &[u8; W])>(
        &mut self,
        pts: &[Pt],
        rows: Range<i32>,
        mut emit: F,
    ) -> Result<(), Error> {
        self.n = 0;
        for i in 0..pts.len() {
            self.push(pts[i], pts[(i + 1) % pts.len()])?;
        }
        for y in rows {
            self.acc = [0; W];
            let (top, bot) = (y * ONE, (y + 1) * ONE);
            for e in &self.edges[..self.n] {
                let (ya, yb) = (e.y0.max(top), e.y1.min(bot));
                if ya < yb {
                    let (xa, xb) = (e.x_at(ya), e.x_at(yb));
                    add_piece(&mut self.acc, (xa, ya), (xb, yb), e.dir);
                }
            }
            let mut sum = 0i64;
            for x in 0..W {
                sum += self.acc[x] as i64;
                let cov = sum.abs().min(FULL);
                self.row[x] = ((cov * 255 + FULL / 2) / FULL) as u8;
            }
            emit(y, &self.row);
        }
        Ok(())
    }

    fn push(&mut self, a: Pt, b: Pt) -> Result<(), Error> {
        if a.1 == b.1 {
            return Ok(());
        }
        let (top, bot, dir) = if a.1 < b.1 { (a, b, 1) } else { (b, a, -1) };
        if self.n == E {
            return Err(Error::Capacity);
        }
        self.edges[self.n] = Edge { x0: top.0, y0: top.1, x1: bot.0, y1: bot.1, dir };
        self.n += 1;
        Ok(())
    }
}

/// Adds the signed area of one segment piece that lies inside a single pixel row.
/// `a` and `b` have `a.y < b.y`. The piece is split at pixel columns; inside
/// column `p` the area right of the piece is `dy * (256 - xr)` and the rest goes
/// to column `p+1`. Columns left of 0 count as fully covering pixel 0.
fn add_piece<const W: usize>(acc: &mut [i32; W], a: Pt, b: Pt, dir: i32) {
    let (xlo, xhi, ylo, yhi) = if a.0 <= b.0 { (a.0, b.0, a.1, b.1) } else { (b.0, a.0, b.1, a.1) };
    let span = (xhi - xlo) as i64;
    let dy_of = |c: i32| -> i64 {
        // y at column coordinate c (linear along the piece)
        ylo as i64 + if span == 0 { 0 } else { (yhi - ylo) as i64 * (c - xlo) as i64 / span }
    };
    for p in (xlo >> 8)..=(xhi >> 8) {
        let c0 = xlo.max(p << 8);
        let c1 = xhi.min((p + 1) << 8);
        let (dy, mid) = if span == 0 {
            ((yhi - ylo) as i64, xlo as i64)
        } else {
            ((dy_of(c1) - dy_of(c0)).abs(), (c0 as i64 + c1 as i64) / 2)
        };
        let d = dy * dir as i64;
        let xr = mid - ((p as i64) << 8); // 0..=256
        if p < 0 {
            acc[0] += (d * ONE as i64) as i32;
        } else if (p as usize) < acc.len() {
            let i = p as usize;
            acc[i] += (d * (ONE as i64 - xr)) as i32;
            if i + 1 < acc.len() {
                acc[i + 1] += (d * xr) as i32;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: usize = 8;
    const H: i32 = 8;

    /// Renders into a small grid for checks.
    fn grid<const E: usize>(pts: &[Pt]) -> Result<[[u8; W]; H as usize], Error> {
        let mut img = [[0u8; W]; H as usize];
        let mut r = Rasterizer::<W, E>::new();
        r.render(pts, 0..H, |y, row| img[y as usize] = *row)?;
        Ok(img)
    }

    fn px(v: i32) -> i32 {
        v * ONE
    }

    #[test]
    fn square_inside_grid_is_255_inside_0_outside() {
        let sq = [(px(2), px(2)), (px(6), px(2)), (px(6), px(6)), (px(2), px(6))];
        let img = grid::<4>(&sq).unwrap();
        for y in 0..H as usize {
            for x in 0..W {
                let inside = (2..6).contains(&x) && (2..6).contains(&y);
                assert_eq!(img[y][x], if inside { 255 } else { 0 }, "px {x},{y}");
            }
        }
    }

    #[test]
    fn square_clipped_at_left_edge() {
        // x from -2 to 2: pixels 0 and 1 are fully covered, the rest is empty.
        let sq = [(px(-2), px(1)), (px(2), px(1)), (px(2), px(3)), (px(-2), px(3))];
        let img = grid::<4>(&sq).unwrap();
        assert_eq!(img[2][0], 255);
        assert_eq!(img[2][1], 255);
        assert_eq!(img[2][2], 0);
        assert_eq!(img[1][0], 255);
    }

    #[test]
    fn half_pixel_edge_gives_128() {
        // Left edge at x = 1.5, right edge at x = 4.0, rows 1..5.
        let r = [(px(1) + 128, px(1)), (px(4), px(1)), (px(4), px(5)), (px(1) + 128, px(5))];
        let img = grid::<4>(&r).unwrap();
        assert_eq!(img[3][1], 128);
        assert_eq!(img[3][2], 255);
        assert_eq!(img[3][3], 255);
        assert_eq!(img[3][4], 0);
    }

    #[test]
    fn triangle_area_matches_analytic() {
        // Non-trivial triangle in 24.8 units; doubled area via shoelace in i64.
        let t: [Pt; 3] = [(77, 180), (1950, 300), (740, 1650)];
        let cross: i64 = (0..3)
            .map(|i| {
                let (a, b) = (t[i], t[(i + 1) % 3]);
                a.0 as i64 * b.1 as i64 - b.0 as i64 * a.1 as i64
            })
            .sum();
        let want = cross.abs() / 2; // in 1/65536 px^2

        let mut total = 0i64;
        let mut r = Rasterizer::<8, 3>::new();
        r.render(&t, 0..8, |_, row| total += row.iter().map(|&c| c as i64).sum::<i64>())
            .unwrap();
        // Each coverage unit is 1/255 px^2.
        let got = total * FULL / 255;
        let tol = FULL / 4; // 0.25 px^2
        assert!((got - want).abs() < tol, "got {got} want {want}");
    }

    #[test]
    fn exceeding_capacity_is_err() {
        let tri = [(px(0), px(0)), (px(4), px(2)), (px(0), px(4))];
        assert_eq!(grid::<2>(&tri), Err(Error::Capacity));
        assert!(grid::<3>(&tri).is_ok());
    }
}
