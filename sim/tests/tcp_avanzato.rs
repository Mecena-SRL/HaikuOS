use haiku_alloc_sim::tcp::{Conn, State, MSS, RTO_TICKS};

fn established_pair() -> (Conn, Conn) {
    let mut srv = Conn::listen(80);
    let (mut cli, syn) = Conn::connect(5000, 80, 100);
    let synack = srv.on_segment(&syn).unwrap();
    let ack = cli.on_segment(&synack).unwrap();
    srv.on_segment(&ack);
    (cli, srv)
}

fn read_all(c: &mut Conn) -> (Vec<u8>, Option<haiku_alloc_sim::tcp::Segment>) {
    let mut buf = [0u8; 2048];
    let (n, ack) = c.read(&mut buf);
    (buf[..n].to_vec(), ack)
}

#[test]
fn syn_perso_viene_ritrasmesso() {
    let mut srv = Conn::listen(80);
    let (mut cli, _lost_syn) = Conn::connect(5000, 80, 100);
    for _ in 0..RTO_TICKS - 1 {
        assert!(cli.tick().is_none());
    }
    let again = cli.tick().expect("SYN ritrasmesso");
    let synack = srv.on_segment(&again).unwrap();
    let ack = cli.on_segment(&synack).unwrap();
    srv.on_segment(&ack);
    assert_eq!(cli.state, State::Established);
    assert_eq!(srv.state, State::Established);
}

#[test]
fn syn_ack_perso_il_syn_ripetuto_lo_rimanda() {
    let mut srv = Conn::listen(80);
    let (mut cli, syn) = Conn::connect(5000, 80, 100);
    let _lost_synack = srv.on_segment(&syn).unwrap(); // perso
    for _ in 0..RTO_TICKS - 1 {
        cli.tick();
    }
    let syn2 = cli.tick().expect("SYN ritrasmesso");
    let synack2 = srv.on_segment(&syn2).expect("il server rimanda il SYN-ACK");
    let ack = cli.on_segment(&synack2).unwrap();
    srv.on_segment(&ack);
    assert_eq!(cli.state, State::Established);
    assert_eq!(srv.state, State::Established);
}

#[test]
fn syn_senza_risposta_chiude_dopo_i_tentativi() {
    let (mut cli, _syn) = Conn::connect(5000, 80, 100);
    for _ in 0..200 {
        cli.tick();
    }
    assert_eq!(cli.state, State::Closed);
}

#[test]
fn congestione_cresce_in_slow_start_e_collassa_al_timeout() {
    let (mut cli, mut srv) = established_pair();
    assert_eq!(cli.cwnd(), MSS);
    let d = cli.send(&[1; MSS]).unwrap();
    let ack = srv.on_segment(&d).unwrap();
    cli.on_segment(&ack);
    assert_eq!(cli.cwnd(), 2 * MSS, "slow start: +1 MSS per segmento confermato");

    let _lost = cli.send(&[2; MSS]).unwrap();
    for _ in 0..cli.rto() {
        if cli.tick().is_some() {
            assert_eq!(cli.cwnd(), MSS);
            return;
        }
    }
    panic!("nessuna ritrasmissione allo scadere del RTO");
}

#[test]
fn rto_si_adatta_alla_misura_e_raddoppia_al_timeout() {
    let (mut cli, mut srv) = established_pair();
    let d = cli.send(b"ping").unwrap();
    cli.tick();
    cli.tick(); // risposta dopo 2 tick
    let ack = srv.on_segment(&d).unwrap();
    cli.on_segment(&ack);
    // prima misura r=2: SRTT=2, RTTVAR=1, RTO=2+4=6
    assert_eq!(cli.rto(), 6);

    let _lost = cli.send(b"pong").unwrap();
    let mut ticks = 0;
    while cli.tick().is_none() {
        ticks += 1;
    }
    assert_eq!(ticks + 1, 6, "timeout dopo RTO tick");
    assert_eq!(cli.rto(), 12, "backoff: RTO raddoppiato");
}

#[test]
fn profilo_gba_totale_ricevuto_corretto() {
    let mut srv = Conn::listen(80);
    srv.rcv_buf = 256;
    let (mut cli, syn) = Conn::connect(5000, 80, 100);
    let synack = srv.on_segment(&syn).unwrap();
    let ack = cli.on_segment(&synack).unwrap();
    srv.on_segment(&ack);

    let data: Vec<u8> = (0..600u32).map(|i| (i % 251) as u8).collect();
    let mut got = Vec::new();
    for chunk in data.chunks(MSS) {
        let seg = cli.send(chunk).unwrap();
        let ack = srv.on_segment(&seg).unwrap();
        cli.on_segment(&ack);
        let (d, upd) = read_all(&mut srv);
        got.extend(d);
        if let Some(u) = upd {
            cli.on_segment(&u);
        }
    }
    assert_eq!(got, data);
}
