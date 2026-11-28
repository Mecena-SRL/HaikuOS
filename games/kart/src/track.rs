//! Circuiti: poligoni chiusi di punti interi. Il segmento `i` va dal punto `i` al `i+1`.
//! Il segmento 0 è dopo la linea del traguardo.

use crate::fx::{int, mul, Fx, ONE};

pub struct Track {
    pub name: &'static str,
    pub pts: &'static [(i32, i32)],
    pub half_w: i32,
    pub laps: u8,
}

/// Punto della pista più vicino a una posizione.
#[derive(Clone, Copy, Default)]
pub struct Near {
    pub seg: usize,
    pub t: Fx,
    pub x: Fx,
    pub y: Fx,
    /// Distanza al quadrato, scala 2^32 (Fx²).
    pub d2: i64,
}

impl Track {
    pub fn n(&self) -> usize {
        self.pts.len()
    }

    fn p(&self, i: usize) -> (Fx, Fx) {
        let (x, y) = self.pts[i % self.pts.len()];
        (int(x), int(y))
    }

    /// Punto sul segmento `seg` alla frazione `t` (0..ONE).
    pub fn point(&self, seg: usize, t: Fx) -> (Fx, Fx) {
        let (ax, ay) = self.p(seg);
        let (bx, by) = self.p(seg + 1);
        (ax + mul(bx - ax, t), ay + mul(by - ay, t))
    }

    /// Segmento più vicino a (x, y): proiezione su ogni segmento, tiene la minima.
    pub fn nearest(&self, x: Fx, y: Fx) -> Near {
        let mut best = Near { d2: i64::MAX, ..Default::default() };
        for i in 0..self.n() {
            let (ax, ay) = self.p(i);
            let (bx, by) = self.p(i + 1);
            let (dx, dy) = (bx - ax, by - ay);
            let (px, py) = (x - ax, y - ay);
            let l2 = dx as i64 * dx as i64 + dy as i64 * dy as i64;
            let dot = px as i64 * dx as i64 + py as i64 * dy as i64;
            let t = if dot <= 0 {
                0
            } else if dot >= l2 {
                ONE
            } else {
                (dot / (l2 >> 16).max(1)) as Fx
            };
            let qx = ax + ((dx as i64 * t as i64) >> 16) as Fx;
            let qy = ay + ((dy as i64 * t as i64) >> 16) as Fx;
            let (ex, ey) = ((x - qx) as i64, (y - qy) as i64);
            let d2 = ex * ex + ey * ey;
            if d2 < best.d2 {
                best = Near { seg: i, t, x: qx, y: qy, d2 };
            }
        }
        best
    }

    /// Soglia fuori pista (distanza² scala 2^32).
    pub fn off_d2(&self) -> i64 {
        let w = (self.half_w as i64) << 16;
        w * w
    }

    /// Distanza oltre la quale il kart viene riportato verso la pista.
    pub fn wall(&self) -> Fx {
        int(2 * self.half_w + 10)
    }
}

const ANELLO: Track = Track {
    name: "Anello",
    pts: &[
        (900, 500), (870, 653), (783, 783), (653, 870), (500, 900), (347, 870), (217, 783),
        (130, 653), (100, 500), (130, 347), (217, 217), (347, 130), (500, 100), (653, 130),
        (783, 217), (870, 347),
    ],
    half_w: 40,
    laps: 3,
};

const SERPENTE: Track = Track {
    name: "Serpente",
    pts: &[
        (100, 100), (400, 100), (500, 250), (600, 100), (900, 100), (900, 400), (700, 500),
        (900, 600), (900, 900), (600, 900), (500, 750), (400, 900), (100, 900), (100, 600),
        (300, 500), (100, 400),
    ],
    half_w: 40,
    laps: 3,
};

const OTTAGONO: Track = Track {
    name: "Ottagono",
    pts: &[
        (300, 100), (700, 100), (900, 300), (900, 700), (700, 900), (300, 900), (100, 700),
        (100, 300),
    ],
    half_w: 40,
    laps: 3,
};

pub static TRACKS: [Track; 3] = [ANELLO, SERPENTE, OTTAGONO];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centerline_is_on_track() {
        let t = &TRACKS[0];
        for seg in 0..t.n() {
            let (x, y) = t.point(seg, ONE / 3);
            assert!(t.nearest(x, y).d2 < 1i64 << 32);
        }
    }

    #[test]
    fn far_point_is_offroad() {
        let t = &TRACKS[0];
        let (x, y) = t.point(0, ONE / 2);
        assert!(t.nearest(x, y + int(200)).d2 > t.off_d2());
    }
}
