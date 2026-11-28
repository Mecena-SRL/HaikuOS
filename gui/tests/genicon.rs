// Icone generative: LOD, gerarchia, limiti nella cella, cache (hit/miss/sostituzione).
use haiku_gui::genicon::{folder, lod_for, Cache, Lod, Prim, MAX_PRIMS};
use haiku_gui::layout::Rect;

fn prims(w: i32, h: i32) -> (usize, Lod, [Prim; MAX_PRIMS]) {
    let mut out = [Prim { r: Rect::new(0, 0, 0, 0), radius: 0, t: haiku_gui::Tok::T0 }; MAX_PRIMS];
    let (n, lod) = folder(Rect::new(0, 0, w, h), &mut out);
    (n, lod, out)
}

#[test]
fn lod_thresholds() {
    assert_eq!(lod_for(Rect::new(0, 0, 24, 24)), Lod::L0);
    assert_eq!(lod_for(Rect::new(0, 0, 32, 32)), Lod::L1);
    assert_eq!(lod_for(Rect::new(0, 0, 48, 40)), Lod::L1); // conta il lato minore
    assert_eq!(lod_for(Rect::new(0, 0, 64, 64)), Lod::L2);
}

#[test]
fn hierarchy_drops_parts_below_threshold() {
    assert_eq!(prims(24, 24).0, 2); // retro + linguetta
    assert_eq!(prims(48, 48).0, 3); // + falda
    assert_eq!(prims(96, 96).0, 4); // + luce
    let (_, _, p) = prims(96, 96);
    assert_eq!(p[0].radius, 12); // raggio = 1/8 del lato
    let (_, _, q) = prims(24, 24);
    assert!(q.iter().take(2).all(|p| p.radius == 0));
}

#[test]
fn primitives_stay_inside_cell() {
    for s in [16, 24, 32, 48, 64, 96, 192] {
        let (n, _, p) = prims(s, s);
        for pr in &p[..n] {
            assert!(pr.r.x >= 0 && pr.r.y >= 0 && pr.r.x + pr.r.w <= s && pr.r.y + pr.r.h <= s, "{s}");
            assert!(pr.r.w > 0 && pr.r.h > 0, "{s}");
        }
    }
}

#[test]
fn cache_hits_and_replaces_on_resize() {
    let mut c: Cache<4> = Cache::new();
    let a = c.folder(0, 48, 48);
    let b = c.folder(0, 48, 48);
    assert_eq!((c.hits, c.misses), (1, 1));
    assert_eq!(a.n, b.n);
    let big = c.folder(0, 96, 96); // schermo più grande: voce sostituita
    assert_eq!((c.hits, c.misses), (1, 2));
    assert_eq!(big.lod, Lod::L2);
    c.folder(1, 24, 24); // altro tipo: slot separato
    c.folder(0, 96, 96);
    assert_eq!((c.hits, c.misses), (2, 3));
}
