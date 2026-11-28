use haiku_alloc_sim::*;

fn p(n: u8) -> ProgramId {
    ProgramId(n)
}

fn setup() -> Monolith<GreedyAgent> {
    let mut m = Monolith::new(GreedyAgent);
    for i in 1..=4 {
        assert!(m.register(p(i)));
    }
    m
}

// Agente non fidato: propone una quota enorme.
struct Overreach;
impl Agent for Overreach {
    fn propose(&mut self, _: &QuotaRequest, _: usize) -> Proposal {
        Proposal::Grant(1 << 20)
    }
}

// Agente non fidato: concede più di quanto richiesto.
struct Inflate;
impl Agent for Inflate {
    fn propose(&mut self, req: &QuotaRequest, _: usize) -> Proposal {
        Proposal::Grant(req.bytes * 2)
    }
}

#[test]
fn massimo_quattro_programmi() {
    let mut m = Monolith::new(GreedyAgent);
    for i in 1..=4 {
        assert!(m.register(p(i)));
    }
    assert!(!m.register(p(5)));
    assert!(!m.register(p(1)));
}

#[test]
fn richieste_oltre_il_limite_negate() {
    let mut m = setup();
    assert_eq!(m.request(p(1), SLOT_BYTES + 1), Decision::Denied(DenyReason::TooLarge));
    assert_eq!(m.request(p(1), 0), Decision::Denied(DenyReason::TooLarge));
    assert_eq!(m.request(p(9), 1024), Decision::Denied(DenyReason::UnknownProgram));
    assert_eq!(m.request(p(1), SLOT_BYTES), Decision::Granted(SLOT_BYTES));
    assert_eq!(m.request(p(1), 1024), Decision::Denied(DenyReason::AlreadyGranted));
}

#[test]
fn riserva_sempre_libera_e_pool_rispettato() {
    let mut m = setup();
    for i in 1..=3 {
        assert_eq!(m.request(p(i), SLOT_BYTES), Decision::Granted(SLOT_BYTES));
    }
    // Il quarto programma non ottiene lo slot riservato.
    assert_eq!(m.request(p(4), 1024), Decision::Denied(DenyReason::NoFreeSlot));
    assert_eq!(m.granted_count(), MAX_PROGRAMS - RESERVED_FREE_SLOTS);
    assert!(m.granted_count() * SLOT_BYTES <= POOL_BYTES);
}

#[test]
fn revoca_libera_lo_slot() {
    let mut m = setup();
    for i in 1..=3 {
        m.request(p(i), 1024);
    }
    assert!(m.revoke(p(1)));
    assert_eq!(m.request(p(4), 1024), Decision::Granted(1024));
    assert_eq!(m.granted_count(), MAX_GRANTS);
}

#[test]
fn agente_non_fidato_negato() {
    let mut m = Monolith::new(Overreach);
    m.register(p(1));
    assert_eq!(m.request(p(1), 1024), Decision::Denied(DenyReason::AgentOverreach));
    assert_eq!(m.granted_count(), 0);

    let mut m = Monolith::new(Inflate);
    m.register(p(1));
    assert_eq!(m.request(p(1), 1024), Decision::Denied(DenyReason::AgentOverreach));
    assert_eq!(m.granted_count(), 0);
}

#[test]
fn messaggi_non_permessi_scartati() {
    let mut m = setup();
    m.allow(p(1), p(2));
    m.request(p(1), 1024);
    m.request(p(2), 1024);
    m.request(p(3), 1024);

    assert_eq!(m.send(p(1), p(2), b"ok"), Ok(()));
    assert_eq!(m.send(p(2), p(1), b"no"), Err(FilterError::NotAllowed));
    assert_eq!(m.send(p(1), p(3), b"no"), Err(FilterError::NotAllowed));
    assert_eq!(m.send(p(1), p(4), b"no"), Err(FilterError::NotAllowed));
    assert_eq!(m.inbox().len(), 1);
}

#[test]
fn messaggi_dimensione_e_slot() {
    let mut m = setup();
    m.allow(p(1), p(2));
    m.request(p(1), 1024);
    m.request(p(2), 1024);

    assert_eq!(m.send(p(1), p(2), &[0u8; MAX_MESSAGE_BYTES]), Ok(()));
    assert_eq!(
        m.send(p(1), p(2), &[0u8; MAX_MESSAGE_BYTES + 1]),
        Err(FilterError::TooLarge)
    );

    m.allow(p(3), p(2));
    assert_eq!(m.send(p(3), p(2), b"x"), Err(FilterError::NoSlot));
}

#[test]
fn messaggio_non_concede_diritti() {
    let mut m = setup();
    m.allow(p(1), p(2));
    m.request(p(1), 1024);
    m.request(p(2), 1024);

    let before = m.grant_of(p(1));
    assert_eq!(m.send(p(1), p(2), b"GRANT 32768 TO 1"), Ok(()));
    assert_eq!(m.grant_of(p(1)), before);
    assert_eq!(m.granted_count(), 2);
}
