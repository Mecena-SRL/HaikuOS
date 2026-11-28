use haiku_alloc_sim::process::*;
use haiku_alloc_sim::{ProgramId, MAX_PROGRAMS};

fn p(n: u8) -> ProgramId {
    ProgramId(n)
}

#[test]
fn tabella_piena_e_duplicati_rifiutati() {
    let mut s = Scheduler::new();
    for i in 1..=MAX_PROGRAMS as u8 {
        assert_eq!(s.spawn(p(i), 1, 2), Ok(()));
    }
    assert_eq!(s.spawn(p(9), 1, 2), Err(SpawnError::Full));
    assert_eq!(s.count(), MAX_PROGRAMS);
}

#[test]
fn duplicato_rifiutato_con_tabella_libera() {
    let mut s = Scheduler::new();
    s.spawn(p(1), 1, 2).unwrap();
    assert_eq!(s.spawn(p(1), 1, 2), Err(SpawnError::Duplicate));
    assert_eq!(s.count(), 1);
}

#[test]
fn budget_zero_rifiutato() {
    let mut s = Scheduler::new();
    assert_eq!(s.spawn(p(1), 1, 0), Err(SpawnError::ZeroBudget));
    assert_eq!(s.count(), 0);
}

#[test]
fn round_robin_a_parita_di_priorita() {
    let mut s = Scheduler::new();
    s.spawn(p(1), 1, 2).unwrap();
    s.spawn(p(2), 1, 2).unwrap();
    let seq: Vec<u8> = (0..8).map(|_| s.tick().unwrap().0).collect();
    assert_eq!(seq, vec![1, 1, 2, 2, 1, 1, 2, 2]);
}

#[test]
fn priorita_alta_prende_la_cpu() {
    let mut s = Scheduler::new();
    s.spawn(p(1), 2, 5).unwrap();
    assert_eq!(s.tick(), Some(p(1)));
    s.spawn(p(2), 0, 5).unwrap();
    assert_eq!(s.tick(), Some(p(2)));
    assert_eq!(s.state_of(p(1)), Some(State::Ready));
}

#[test]
fn bloccato_cede_la_cpu_e_risveglio_a_parita_non_ruba() {
    let mut s = Scheduler::new();
    s.spawn(p(1), 1, 10).unwrap();
    s.spawn(p(2), 1, 10).unwrap();
    assert_eq!(s.tick(), Some(p(1)));
    assert!(s.block(p(1)));
    assert_eq!(s.tick(), Some(p(2)));
    assert!(s.wake(p(1)));
    // Stessa priorità: B continua finché non esaurisce il budget.
    assert_eq!(s.tick(), Some(p(2)));
}

#[test]
fn risveglio_con_priorita_alta_preempta() {
    let mut s = Scheduler::new();
    s.spawn(p(1), 0, 10).unwrap();
    s.spawn(p(2), 1, 10).unwrap();
    assert_eq!(s.tick(), Some(p(1)));
    s.block(p(1));
    assert_eq!(s.tick(), Some(p(2)));
    s.wake(p(1));
    assert_eq!(s.tick(), Some(p(1)));
}

#[test]
fn tutti_bloccati_significa_idle() {
    let mut s = Scheduler::new();
    s.spawn(p(1), 1, 3).unwrap();
    s.block(p(1));
    assert_eq!(s.tick(), None);
    assert_eq!(s.state_of(p(1)), Some(State::Blocked));
}

#[test]
fn exit_libera_lo_slot() {
    let mut s = Scheduler::new();
    for i in 1..=MAX_PROGRAMS as u8 {
        s.spawn(p(i), 1, 2).unwrap();
    }
    assert!(s.exit(p(2)));
    assert_eq!(s.count(), MAX_PROGRAMS - 1);
    assert_eq!(s.spawn(p(9), 1, 2), Ok(()));
    assert!(!s.exit(p(2)));
}

#[test]
fn exit_del_processo_in_esecuzione_ripartono_gli_altri() {
    let mut s = Scheduler::new();
    s.spawn(p(1), 1, 5).unwrap();
    s.spawn(p(2), 1, 5).unwrap();
    assert_eq!(s.tick(), Some(p(1)));
    assert!(s.exit(p(1)));
    assert_eq!(s.tick(), Some(p(2)));
}

#[test]
fn stato_non_valido_non_cambia_nulla() {
    let mut s = Scheduler::new();
    s.spawn(p(1), 1, 3).unwrap();
    assert!(!s.wake(p(1)), "un processo pronto non si risveglia");
    assert!(!s.block(p(7)));
    assert!(!s.wake(p(7)));
}
