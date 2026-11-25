use haiku_alloc_sim::tcp::{Conn, Segment, State, RTO_TICKS};

fn connected() -> (Conn, Conn) {
    let mut srv = Conn::listen(80);
    let (mut cli, syn) = Conn::connect(5000, 80, 100);
    let synack = srv.on_segment(&syn).unwrap();
    let ack = cli.on_segment(&synack).unwrap();
    srv.on_segment(&ack);
    assert_eq!(cli.state, State::Established);
    assert_eq!(srv.state, State::Established);
    (cli, srv)
}

/// Legge tutto ciò che è in RX.
fn read_all(c: &mut Conn) -> (Vec<u8>, Option<Segment>) {
    let mut buf = [0u8; 2048];
    let (n, ack) = c.read(&mut buf);
    (buf[..n].to_vec(), ack)
}

#[test]
fn ritrasmissione_dopo_perdita() {
    let (mut cli, mut srv) = connected();
    let lost = cli.send(b"abc").unwrap(); // non consegnato: perso sul filo
    assert_eq!(cli.in_flight(), 3);
    // prima del timeout nulla
    for _ in 0..RTO_TICKS - 1 {
        assert!(cli.tick().is_none());
    }
    let again = cli.tick().expect("ritrasmissione allo scadere del RTO");
    assert_eq!(again.seq, lost.seq);
    assert_eq!(again.data(), b"abc");
    // la ritrasmissione arriva e viene confermata
    let ack = srv.on_segment(&again).unwrap();
    assert_eq!(srv.received(), b"abc");
    cli.on_segment(&ack);
    assert_eq!(cli.in_flight(), 0);
    assert!(cli.tick().is_none());
}

#[test]
fn duplicato_non_raddoppia_i_dati() {
    let (mut cli, mut srv) = connected();
    let d = cli.send(b"abc").unwrap();
    srv.on_segment(&d);
    let dup = srv.on_segment(&d).unwrap(); // stesso segmento due volte
    assert_eq!(srv.received(), b"abc");
    assert_eq!(dup.ack, d.seq.wrapping_add(3));
}

#[test]
fn finestra_blocca_il_mittente_e_la_lettura_la_riapre() {
    let (mut cli, mut srv) = connected();
    srv.rcv_buf = 4;
    let d = cli.send(b"abcd").unwrap();
    let ack = srv.on_segment(&d).unwrap();
    assert_eq!(ack.wnd, 0); // buffer pieno
    cli.on_segment(&ack);
    assert_eq!(cli.in_flight(), 0);
    assert!(cli.send(b"e").is_none(), "finestra a zero: nessun invio");

    let (data, upd) = read_all(&mut srv);
    assert_eq!(data, b"abcd");
    let upd = upd.expect("la lettura annuncia la finestra aperta");
    assert_eq!(upd.wnd, 4);
    cli.on_segment(&upd);
    assert!(cli.send(b"e").is_some());
}

#[test]
fn finestra_limita_i_byte_in_volo() {
    let (mut cli, mut srv) = connected();
    srv.rcv_buf = 4;
    let d = cli.send(b"abcd").unwrap();
    let ack = srv.on_segment(&d).unwrap();
    cli.on_segment(&ack);
    // la finestra annunciata è 0 dopo il primo invio: un secondo invio non entra
    assert!(cli.send(b"x").is_none());
}
