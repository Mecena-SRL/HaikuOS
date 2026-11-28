use haiku_alloc_sim::fence::Fence;
use haiku_alloc_sim::gpu_ring::*;

#[test]
fn anello_consuma_solo_al_campanello() {
    let mut r: Ring<u32, 4> = Ring::new();
    r.push(1).unwrap();
    r.push(2).unwrap();
    let mut visti = Vec::new();
    assert_eq!(r.consume(|c| visti.push(c)), 0);
    r.ring();
    assert_eq!(r.consume(|c| visti.push(c)), 2);
    assert_eq!(visti, [1, 2]);
    assert_eq!(r.pending(), 0);
}

#[test]
fn anello_pieno_rifiuta_e_avvolge_gli_indici() {
    let mut r: Ring<u8, 2> = Ring::new();
    for round in 0..5u8 {
        r.push(round).unwrap();
        r.push(round).unwrap();
        assert_eq!(r.push(0), Err(RingError::Full));
        r.ring();
        assert_eq!(r.consume(|_| {}), 2);
    }
}

#[test]
fn fence_segnala_solo_cio_che_e_stato_inviato() {
    let mut f = Fence::new();
    let a = f.next_seq();
    let b = f.next_seq();
    f.signal(b + 10);
    assert!(f.is_done(a) && f.is_done(b));
    assert!(!f.is_done(b + 1));
}

#[test]
fn fence_non_torna_indietro() {
    let mut f = Fence::new();
    let a = f.next_seq();
    let b = f.next_seq();
    f.signal(b);
    f.signal(a);
    assert_eq!(f.completed(), b);
}

#[test]
fn compositore_attende_fence_dopo_il_campanello() {
    let mut r: Ring<u64, 8> = Ring::new();
    let mut f = Fence::new();
    let mut seq = 0;
    for cmd in [10u64, 20, 30] {
        r.push(cmd).unwrap();
        seq = f.next_seq();
    }
    assert!(!f.is_done(seq));
    r.ring();
    let mut ultimo = 0;
    r.consume(|c| ultimo = c);
    f.signal(seq);
    assert_eq!(ultimo, 30);
    assert!(f.is_done(seq));
}
