use haiku_alloc_sim::tcp::{Conn, Segment, MSS};

fn established_pair() -> (Conn, Conn) {
    let mut srv = Conn::listen(80);
    let (mut cli, syn) = Conn::connect(5000, 80, 100);
    let synack = srv.on_segment(&syn).unwrap();
    let ack = cli.on_segment(&synack).unwrap();
    srv.on_segment(&ack);
    (cli, srv)
}

#[test]
fn fuori_ordine_si_tiene_e_si_riassembla() {
    let (mut cli, mut srv) = established_pair();
    let a = cli.send(b"aaaa").unwrap();
    let b = cli.send(b"bbbb").unwrap();
    let c = cli.send(b"cccc").unwrap();
    srv.on_segment(&c); // arriva prima
    assert!(srv.received.is_empty());
    srv.on_segment(&a);
    assert_eq!(srv.received, b"aaaa");
    srv.on_segment(&b); // riempie il buco: esce anche c dal buffer ooo
    let (data, _) = srv.read();
    assert_eq!(data, b"aaaabbbbcccc");
}

#[test]
fn ack_fuori_ordine_porta_i_blocchi_sack() {
    let (mut cli, mut srv) = established_pair();
    let a = cli.send(b"aaaa").unwrap();
    let _b = cli.send(b"bbbb").unwrap();
    let c = cli.send(b"cccc").unwrap();
    let ack_c = srv.on_segment(&c).unwrap();
    assert_eq!(ack_c.ack, a.seq, "conferma ciò che manca: il primo byte non ricevuto");
    assert_eq!(ack_c.sack, vec![(c.seq, c.seq + 4)]);
    let ack_d = srv.on_segment(&cli.send(b"dddd").unwrap()).unwrap();
    assert_eq!(ack_d.sack, vec![(c.seq, c.seq + 8)], "blocchi contigui si fondono");
}

#[test]
fn sack_sopravvive_al_codifica_e_decodifica() {
    let s = Segment {
        src_port: 1,
        dst_port: 2,
        seq: 10,
        ack: 20,
        flags: 0x10,
        wnd: 300,
        sack: vec![(100, 150), (200, 260)],
        payload: b"hi".to_vec(),
    };
    let raw = s.encode([1, 2, 3, 4], [5, 6, 7, 8]);
    assert_eq!(Segment::parse(&raw, [1, 2, 3, 4], [5, 6, 7, 8]).unwrap(), s);
}

#[test]
fn ritrasmissione_veloce_dopo_tre_ack_duplicati_senza_timeout() {
    let (mut cli, mut srv) = established_pair();
    // far crescere la finestra di congestione
    for _ in 0..2 {
        let d = cli.send(&[0; MSS]).unwrap();
        let a = srv.on_segment(&d).unwrap();
        cli.on_segment(&a);
        let (_, u) = srv.read();
        if let Some(u) = u {
            cli.on_segment(&u);
        }
    }
    let segs: Vec<Segment> = (0..5u8).map(|i| cli.send(&[i; 32]).unwrap()).collect();
    let mut acks = vec![];
    for (i, s) in segs.iter().enumerate() {
        if i == 1 {
            continue; // perso
        }
        acks.push(srv.on_segment(s).unwrap());
    }
    for a in &acks {
        cli.on_segment(a);
    }
    let r = cli.take_fast_rexmit().expect("tre ACK duplicati: ritrasmissione veloce");
    assert_eq!(r.seq, segs[1].seq);
    let a2 = srv.on_segment(&r).unwrap();
    cli.on_segment(&a2);
    let (data, _) = srv.read();
    let mut expected = vec![];
    for i in 0..5u8 {
        expected.extend_from_slice(&[i; 32]);
    }
    assert_eq!(&data[data.len() - 160..], &expected[..]);
    assert_eq!(cli.in_flight(), 0);
}

#[test]
fn sack_evita_di_ritrasmettere_i_segmenti_gia_ricevuti() {
    let (mut cli, mut srv) = established_pair();
    let segs: Vec<Segment> = (0..4u8).map(|i| cli.send(&[i; 8]).unwrap()).collect();
    // il primo si perde, gli altri tre arrivano e sono SACKati
    for s in &segs[1..] {
        let a = srv.on_segment(s).unwrap();
        cli.on_segment(&a);
    }
    // al timeout si rimanda solo il primo, non i SACKati
    let mut rexmit = vec![];
    for _ in 0..10 {
        rexmit = cli.tick();
        if !rexmit.is_empty() {
            break;
        }
    }
    assert_eq!(rexmit.len(), 1);
    assert_eq!(rexmit[0].seq, segs[0].seq);
}
