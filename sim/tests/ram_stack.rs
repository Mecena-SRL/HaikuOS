//! Misura della RAM dello stack sull'host (64 bit). Sulla GBA (ARM7, 32 bit) le dimensioni
//! di puntatori e Vec sono circa la metà: il dato GBA è una stima da verificare.
use haiku_alloc_sim::ip::Packet;
use haiku_alloc_sim::net::{Frame, NetLink};
use haiku_alloc_sim::tcp::{Conn, Segment, RCV_BUF};
use std::mem::size_of;

#[test]
fn misura_ram_stack_host() {
    let conn = size_of::<Conn>();
    let seg = size_of::<Segment>();
    let pkt = size_of::<Packet>();
    let frame = size_of::<Frame>();
    let link = size_of::<NetLink>();
    println!("host bytes: Conn={conn} Segment={seg} Packet={pkt} Frame={frame} NetLink={link} rcv_buf={RCV_BUF}");
    // Per una connessione: struttura + buffer di ricezione + segmenti in volo (al più la finestra).
    let per_conn_host = conn + RCV_BUF;
    println!("host per connessione (struttura + buffer RX, senza segmenti in volo): {per_conn_host}");
    // Limite di sicurezza: se cresce oltre questo, la scelta di design va rivista.
    assert!(per_conn_host < 2 * 1024, "stack troppo grande: {per_conn_host} byte");
}
