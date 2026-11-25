//! Misura della RAM dello stack. Nessun Vec né BTreeMap: la dimensione è fissa e nota a compilazione.
use haiku_alloc_sim::ip::Packet;
use haiku_alloc_sim::net::{Frame, NetLink};
use haiku_alloc_sim::tcp::{Conn, ConnN, Segment, MSS, OOO_SLOTS, OUT_SLOTS, RCV_BUF};
use std::mem::size_of;

#[test]
fn misura_ram_stack_host_e_profilo_gba() {
    let host = size_of::<ConnN<RCV_BUF>>();
    let gba = size_of::<ConnN<256>>();
    println!("host: Conn<1024>={host} B  Segment={} B  Packet={} B", size_of::<Segment>(), size_of::<Packet>());
    println!("profilo GBA: ConnN<256>={gba} B (RX+TX 512, {OUT_SLOTS} slot ACK, {OOO_SLOTS} slot fuori ordine da {MSS} B)");
    println!("rete (Vec, solo host): Frame={} B NetLink={} B", size_of::<Frame>(), size_of::<NetLink>());
    assert_eq!(size_of::<Conn>(), host);
    // Limite del profilo GBA: se cresce oltre 2 KB la scelta dei buffer va rivista.
    assert!(gba < 2 * 1024, "profilo GBA troppo grande: {gba} B");
}
