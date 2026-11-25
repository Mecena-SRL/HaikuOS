use haiku_alloc_sim::crypto::{CryptoError, Vault};
use haiku_alloc_sim::ip::{IpError, Packet, PROTO_TCP};
use haiku_alloc_sim::memory::{MemError, MemoryKernel, Pool};
use haiku_alloc_sim::net::{Frame, NetLink};
use haiku_alloc_sim::resource::{pick_ready, Arch, Kind, Manifest, Microkernel, Status};
use haiku_alloc_sim::storage::{Device, StorageError};
use haiku_alloc_sim::tcp::{Conn, Segment, State, TcpError};
use haiku_alloc_sim::gpu::{Gpu, GPU_GENERIC};
use haiku_alloc_sim::net::BROADCAST;

const A_IP: [u8; 4] = [10, 0, 0, 1];
const B_IP: [u8; 4] = [10, 0, 0, 2];
const A_MAC: [u8; 6] = [2, 0, 0, 0, 0, 1];
const B_MAC: [u8; 6] = [2, 0, 0, 0, 0, 2];

// ---- fermo ----
#[test]
fn memoria_fermata_non_alloca_ne_scrive() {
    let mut m = MemoryKernel::new(2, 0, 4, 1);
    m.set_quota(0, 2);
    let h = m.alloc(0, Pool::Ram).unwrap();
    m.halt();
    assert_eq!(m.status(), Status::Stopped);
    assert_eq!(m.alloc(0, Pool::Ram), Err(MemError::Halted));
    assert_eq!(m.write(h, 0, 0, &[1]), Err(MemError::Halted));
    assert!(m.read(h, 0).is_ok()); // lettura resta
}

#[test]
fn storage_fermo_non_scrive_ma_legge() {
    let mut d = Device::new(1, 4);
    d.write(0, &[1, 2, 3, 4]).unwrap();
    d.halt();
    assert_eq!(d.status(), Status::Stopped);
    assert_eq!(d.write(0, &[0; 4]), Err(StorageError::Halted));
    assert_eq!(d.read(0).unwrap(), &[1, 2, 3, 4]);
}

#[test]
fn crittografia_ferma_non_cifra() {
    let mut v = Vault::new([9; 16], 1, 256);
    let s = v.seal(0, b"x").unwrap();
    v.halt();
    assert_eq!(v.status(), Status::Stopped);
    assert_eq!(v.seal(0, b"y"), Err(CryptoError::Halted));
    assert_eq!(v.open(0, &s).unwrap(), b"x");
}

#[test]
fn monolite_sceglie_la_gpu_pronta() {
    let mut g1 = Gpu::new(4, 1);
    let g2 = Gpu::new(4, 1);
    let ks: Vec<&dyn Microkernel> = vec![&g1, &g2];
    assert_eq!(pick_ready(&ks, Kind::Gpu), Some(0));
    let _ = &mut g1; // Gpu non ha halt: testiamo con un Driver fermo
    let m = Manifest { name: "x", kind: Kind::Gpu, archs: &[Arch::Gba], required: true, bytes: 1, queue_depth: 1 };
    let mut d = haiku_alloc_sim::resource::Driver::new(m);
    d.stop();
    let ks2: Vec<&dyn Microkernel> = vec![&d, &g2];
    assert_eq!(pick_ready(&ks2, Kind::Gpu), Some(1));
    assert_eq!(pick_ready(&ks2, Kind::Audio), None);
    assert_eq!(GPU_GENERIC.kind, Kind::Gpu);
}

// ---- IPv4 ----
#[test]
fn ipv4_round_trip_e_checksum() {
    let p = Packet { src: A_IP, dst: B_IP, proto: PROTO_TCP, payload: vec![1, 2, 3] };
    let raw = p.encode().unwrap();
    assert_eq!(Packet::parse(&raw).unwrap(), p);
    let mut bad = raw.clone();
    bad[8] ^= 1; // TTL alterato
    assert_eq!(Packet::parse(&bad), Err(IpError::BadChecksum));
}

// ---- TCP ----
fn pump(a: &mut Conn, b: &mut Conn, first: Segment) {
    // a→b, poi risposte finché non si fermano
    let mut to_b = vec![first];
    let mut to_a = vec![];
    for _ in 0..10 {
        for s in to_b.drain(..) {
            if let Some(r) = b.on_segment(&s) { to_a.push(r); }
        }
        for s in to_a.drain(..) {
            if let Some(r) = a.on_segment(&s) { to_b.push(r); }
        }
        if to_a.is_empty() && to_b.is_empty() { break; }
    }
}

#[test]
fn tcp_handshake_e_dati_in_ordine() {
    let mut srv = Conn::listen(80);
    let (mut cli, syn) = Conn::connect(5000, 80, 100);
    pump(&mut cli, &mut srv, syn);
    assert_eq!(cli.state, State::Established);
    assert_eq!(srv.state, State::Established);
    let d = cli.send(b"hello").unwrap();
    let ack = srv.on_segment(&d).unwrap();
    assert_eq!(srv.received, b"hello");
    cli.on_segment(&ack);
    let d2 = cli.send(b" world").unwrap();
    srv.on_segment(&d2);
    assert_eq!(srv.received, b"hello world");
}

#[test]
fn tcp_segmento_fuori_ordine_ignorato_e_porta_sbagliata() {
    let mut srv = Conn::listen(80);
    let (mut cli, syn) = Conn::connect(5000, 80, 100);
    pump(&mut cli, &mut srv, syn);
    let mut d = cli.send(b"abc").unwrap();
    let in_order = d.seq;
    d.seq = d.seq.wrapping_add(10); // fuori ordine: scartato, risponde con ACK di ciò che manca
    let dup = srv.on_segment(&d).unwrap();
    assert_eq!(dup.ack, in_order);
    assert!(srv.received.is_empty());
    d.dst_port = 81; // porta sbagliata
    assert!(srv.on_segment(&d).is_none());
}

#[test]
fn tcp_fin_chiude_il_lato_remoto() {
    let mut srv = Conn::listen(80);
    let (mut cli, syn) = Conn::connect(5000, 80, 100);
    pump(&mut cli, &mut srv, syn);
    let fin = cli.close().unwrap();
    srv.on_segment(&fin);
    assert_eq!(srv.state, State::CloseWait);
    assert_eq!(cli.state, State::FinWait);
}

#[test]
fn tcp_checksum_su_pseudo_intestazione() {
    let s = Segment { src_port: 1, dst_port: 2, seq: 3, ack: 4, flags: 0x10, wnd: 512, payload: b"xy".to_vec() };
    let raw = s.encode(A_IP, B_IP);
    assert_eq!(Segment::parse(&raw, A_IP, B_IP).unwrap(), s);
    // stesso segmento con un IP diverso: il checksum non torna.
    // (Scambiare sorgente e destinazione non basta: la somma è commutativa.)
    assert_eq!(Segment::parse(&raw, [10, 0, 0, 9], B_IP), Err(TcpError::BadChecksum));
    let mut bad = raw.clone();
    bad[4] ^= 1;
    assert_eq!(Segment::parse(&bad, A_IP, B_IP), Err(TcpError::BadChecksum));
}

// ---- stack completo: TCP su IP su ethernet ----
#[test]
fn tcp_attraverso_ip_ed_ethernet() {
    let mut na = NetLink::new(A_MAC, 8);
    let mut nb = NetLink::new(B_MAC, 8);
    let mut srv = Conn::listen(80);
    let (mut cli, syn) = Conn::connect(5000, 80, 100);

    let wire = |seg: &Segment, src: [u8; 4], dst: [u8; 4], from: &mut NetLink, to_mac: [u8; 6], to: &mut NetLink| {
        let ip = Packet { src, dst, proto: PROTO_TCP, payload: seg.encode(src, dst) };
        let f = Frame { dst: to_mac, src: [0; 6], ethertype: 0x0800, payload: ip.encode().unwrap() };
        from.send(f).unwrap();
        let out = from.next_tx().unwrap();
        to.receive(&out.encode().unwrap()).unwrap();
    };
    let unwire = |to: &mut NetLink, my_ip: [u8; 4], from_ip: [u8; 4]| -> Vec<Segment> {
        let mut v = vec![];
        while let Some(f) = to.next_rx() {
            let ip = Packet::parse(&f.payload).unwrap();
            assert_eq!(ip.dst, my_ip);
            v.push(Segment::parse(&ip.payload, from_ip, my_ip).unwrap());
        }
        v
    };

    wire(&syn, A_IP, B_IP, &mut na, B_MAC, &mut nb);
    for s in unwire(&mut nb, B_IP, A_IP) {
        if let Some(r) = srv.on_segment(&s) { wire(&r, B_IP, A_IP, &mut nb, A_MAC, &mut na); }
    }
    for s in unwire(&mut na, A_IP, B_IP) {
        if let Some(r) = cli.on_segment(&s) { wire(&r, A_IP, B_IP, &mut na, B_MAC, &mut nb); }
    }
    for s in unwire(&mut nb, B_IP, A_IP) { srv.on_segment(&s); }
    assert_eq!(cli.state, State::Established);
    assert_eq!(srv.state, State::Established);

    let d = cli.send(b"ip+eth").unwrap();
    wire(&d, A_IP, B_IP, &mut na, B_MAC, &mut nb);
    for s in unwire(&mut nb, B_IP, A_IP) { srv.on_segment(&s); }
    assert_eq!(srv.received, b"ip+eth");
    let _ = BROADCAST;
}
