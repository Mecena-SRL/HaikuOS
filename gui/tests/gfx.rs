//! Primitive `gfx`: ritaglio, riempimento, copia, zone sporche.
use haiku_gui::gfx::{self, Dirty};
use haiku_gui::layout::Rect;

#[test]
fn ritaglio_e_intersezione() {
    assert_eq!(gfx::clip(Rect::new(-5, -5, 10, 10), 100, 100), Some(Rect::new(0, 0, 5, 5)));
    assert_eq!(gfx::clip(Rect::new(98, 90, 10, 10), 100, 100), Some(Rect::new(98, 90, 2, 10)));
    assert_eq!(gfx::clip(Rect::new(100, 0, 5, 5), 100, 100), None);
    assert_eq!(gfx::intersect(Rect::new(0, 0, 10, 10), Rect::new(5, 5, 10, 10)), Some(Rect::new(5, 5, 5, 5)));
    assert_eq!(gfx::intersect(Rect::new(0, 0, 4, 4), Rect::new(4, 0, 4, 4)), None);
}

#[test]
fn riempi_solo_il_rettangolo() {
    let (w, h) = (8usize, 6usize);
    let mut b = vec![0u32; w * h];
    gfx::fill(&mut b, w, Rect::new(2, 1, 3, 2), 7);
    for y in 0..h {
        for x in 0..w {
            let inside = (2..5).contains(&x) && (1..3).contains(&y);
            assert_eq!(b[y * w + x], if inside { 7 } else { 0 }, "({x},{y})");
        }
    }
}

#[test]
fn copia_solo_il_rettangolo() {
    let (w, h) = (8usize, 4usize);
    let src: Vec<u32> = (0..(w * h) as u32).collect();
    let mut dst = vec![9u32; w * h];
    gfx::copy(&mut dst, &src, w, Rect::new(6, 1, 2, 2));
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let inside = x >= 6 && (1..3).contains(&y);
            assert_eq!(dst[i], if inside { src[i] } else { 9 });
        }
    }
}

#[test]
fn zone_sporche_si_fondono_e_non_perdono_nulla() {
    let mut d: Dirty<4> = Dirty::new();
    d.add(Rect::new(0, 0, 10, 10));
    d.add(Rect::new(5, 5, 10, 10)); // si sovrappone: una sola zona
    assert_eq!(d.rects(), &[Rect::new(0, 0, 15, 15)]);
    d.add(Rect::new(100, 100, 1, 1));
    assert_eq!(d.rects().len(), 2);
    d.add(Rect::new(0, 0, 0, 5)); // vuota: ignorata
    assert_eq!(d.rects().len(), 2);
    // Oltre N zone separate: tutto si fonde in un bounding box che contiene ogni zona.
    for k in 0..6 {
        d.add(Rect::new(200 + 10 * k, 0, 2, 2));
    }
    assert!(d.rects().len() <= 4);
    let bb = d.rects().iter().copied().reduce(|a, b| {
        let x1 = (a.x + a.w).max(b.x + b.w);
        let y1 = (a.y + a.h).max(b.y + b.h);
        Rect::new(a.x.min(b.x), a.y.min(b.y), x1 - a.x.min(b.x), y1 - a.y.min(b.y))
    });
    let bb = bb.unwrap();
    assert!(bb.x <= 0 && bb.y <= 0 && bb.x + bb.w >= 252 && bb.y + bb.h >= 100);
    d.clear();
    assert!(d.rects().is_empty());
}
