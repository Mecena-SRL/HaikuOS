// Animazioni: easing, tween, valore reattivo, budget per frame, transizione e vista spostata.
mod common;
use common::Buf;
use haiku_gui::anim::{ease, Budget, Ease, Reactive, Shifted, Slide, Tween, FRAME_US};
use haiku_gui::layout::Rect;
use haiku_gui::render::Canvas;
use haiku_gui::Tok;

#[test]
fn easing_estremi_e_monotonia() {
    for e in [Ease::Linear, Ease::InQuad, Ease::OutQuad, Ease::InOutQuad, Ease::OutCubic] {
        assert_eq!(ease(e, 0), 0, "{e:?} parte da 0");
        assert_eq!(ease(e, 1000), 1000, "{e:?} arriva a 1000");
        let mut prev = 0;
        for t in 0..=1000 {
            let v = ease(e, t);
            assert!(v >= prev, "{e:?} non monotona a t={t}");
            prev = v;
        }
    }
}

#[test]
fn easing_meta_e_forme() {
    assert_eq!(ease(Ease::Linear, 500), 500);
    assert_eq!(ease(Ease::InQuad, 500), 250);
    assert_eq!(ease(Ease::OutQuad, 500), 750);
    assert_eq!(ease(Ease::InOutQuad, 500), 500);
    assert_eq!(ease(Ease::OutCubic, 500), 875);
    assert_eq!(ease(Ease::OutCubic, 2000), 1000, "t fuori range è clampato");
}

#[test]
fn tween_interpola_e_si_ferma() {
    let tw = Tween { from: 0, to: 100, start: 1000, dur: 200, ease: Ease::Linear };
    assert_eq!(tw.value(500), 0, "prima dell'inizio vale from");
    assert_eq!(tw.value(1100), 50);
    assert_eq!(tw.value(1200), 100);
    assert_eq!(tw.value(9999), 100);
    assert!(!tw.finished(1199));
    assert!(tw.finished(1200));
}

#[test]
fn tween_di_durata_zero_salta_al_bersaglio() {
    let tw = Tween { from: 5, to: 9, start: 0, dur: 0, ease: Ease::OutQuad };
    assert_eq!(tw.value(0), 9);
}

#[test]
fn reattivo_riparte_dal_valore_visualizzato() {
    let mut r = Reactive::new(0);
    r.set(100, 0, 100, Ease::Linear);
    assert_eq!(r.get(50), 50);
    // Cambio di bersaglio a metà: il nuovo tween parte da 50, non da 0.
    r.set(0, 50, 100, Ease::Linear);
    assert_eq!(r.get(50), 50);
    assert_eq!(r.get(100), 25);
    assert!(!r.settled(100));
    assert!(r.settled(150));
    assert_eq!(r.get(150), 0);
}

#[test]
fn reattivo_stesso_bersaglio_non_riavvia() {
    let mut r = Reactive::new(0);
    r.set(100, 0, 100, Ease::Linear);
    r.set(100, 60, 100, Ease::Linear);
    assert_eq!(r.get(50), 50, "l'animazione originale continua");
}

#[test]
fn budget_conta_frame_sforati_e_media() {
    let mut b = Budget::new(FRAME_US);
    b.record(10_000);
    b.record(20_000);
    b.record(FRAME_US);
    assert_eq!(b.frames, 3);
    assert_eq!(b.over, 1, "solo 20 ms sfora 16.667 µs");
    assert_eq!(b.max_us, 20_000);
    assert_eq!(b.last_us, FRAME_US);
    assert_eq!(b.avg_us(), (10_000 + 20_000 + FRAME_US) / 3);
    assert_eq!(b.headroom_us(), 0);
}

#[test]
fn budget_vuoto_e_headroom() {
    let mut b = Budget::new(FRAME_US);
    assert_eq!(b.avg_us(), 0);
    b.record(4_000);
    assert_eq!(b.headroom_us(), FRAME_US - 4_000);
    assert_eq!(b.over, 0);
}

#[test]
fn slide_sposta_vecchia_e_nuova() {
    let s = Slide::start(240, 0, 100);
    assert_eq!(s.offsets(0), (0, 240));
    assert_eq!(s.offsets(100), (-240, 0));
    assert!(!s.finished(99));
    assert!(s.finished(100));
    let (a, b) = s.offsets(50);
    assert_eq!(b - a, 240, "le due schermate coprono sempre una larghezza");
}

#[test]
fn vista_spostata_trasla_fill_e_ritaglia() {
    let mut b = Buf::new(10, 4);
    {
        let mut v = Shifted { inner: &mut b, dx: 3, dy: 1 };
        v.fill(Rect::new(0, 0, 2, 2), Tok::T1);
        v.text(0, 0, "ab", Tok::T0);
    }
    assert_eq!(b.at(3, 1), Tok::T1);
    assert_eq!(b.at(4, 2), Tok::T1);
    assert_eq!(b.at(5, 2), Tok::T0);
    assert_eq!(b.at(2, 1), Tok::T0);
    assert_eq!(b.texts, vec!["ab"]);
}

#[test]
fn vista_spostata_con_offset_negativo_ritaglia_il_bordo() {
    let mut b = Buf::new(4, 1);
    {
        let mut v = Shifted { inner: &mut b, dx: -3, dy: 0 };
        v.fill(Rect::new(0, 0, 4, 1), Tok::T2);
    }
    assert_eq!(b.at(0, 0), Tok::T2);
    assert_eq!(b.at(1, 0), Tok::T0);
}
