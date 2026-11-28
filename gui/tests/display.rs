//! Modello schermo: buffer con handle, coda di flip, piani.
use haiku_gui::display::{Buffer, Buffers, Error, Flip, Pipe};
use haiku_gui::layout::Rect;

fn b(w: u32) -> Buffer {
    Buffer { w, h: 2, pitch: w * 4 }
}

#[test]
fn riferimenti_liberano_solo_all_ultimo() {
    let mut t: Buffers<4> = Buffers::new();
    let h = t.alloc(b(8)).unwrap();
    t.retain(h).unwrap();
    assert_eq!(t.release(h), Ok(false));
    assert_eq!(t.release(h), Ok(true));
    assert_eq!(t.get(h), Err(Error::BadHandle));
}

#[test]
fn handle_vecchio_rifiutato_dopo_il_riuso() {
    let mut t: Buffers<1> = Buffers::new();
    let old = t.alloc(b(8)).unwrap();
    t.release(old).unwrap();
    let new = t.alloc(b(16)).unwrap();
    assert_eq!(t.get(old), Err(Error::BadHandle));
    assert_eq!(t.get(new).unwrap().w, 16);
}

#[test]
fn tabella_piena() {
    let mut t: Buffers<2> = Buffers::new();
    t.alloc(b(1)).unwrap();
    t.alloc(b(2)).unwrap();
    assert_eq!(t.alloc(b(3)), Err(Error::Full));
}

#[test]
fn flip_solo_al_vsync_e_un_solo_slot() {
    let mut t: Buffers<4> = Buffers::new();
    let (a, c) = (t.alloc(b(1)).unwrap(), t.alloc(b(2)).unwrap());
    let mut f = Flip::new();
    assert_eq!(f.vblank(), None);
    f.submit(a).unwrap();
    assert_eq!(f.submit(c), Err(Error::Busy));
    assert_eq!(f.shown(), None); // niente flip prima del vsync
    assert_eq!(f.vblank(), None); // primo flip: nulla da restituire
    assert_eq!(f.shown(), Some(a));
    f.submit(c).unwrap();
    assert_eq!(f.vblank(), Some(a)); // il vecchio buffer torna libero
}

#[test]
fn piani_a_dimensione_fissa() {
    let mut p: Pipe<2> = Pipe::new(1920, 1080);
    let mut t: Buffers<1> = Buffers::new();
    let h = t.alloc(b(4)).unwrap();
    p.set(0, Some(h), Rect::new(0, 0, 1920, 1080)).unwrap();
    assert_eq!(p.plane(0).unwrap().src, Some(h));
    assert_eq!(p.plane(1).unwrap().src, None);
    assert_eq!(p.set(2, None, Rect::new(0, 0, 0, 0)), Err(Error::BadHandle));
}
