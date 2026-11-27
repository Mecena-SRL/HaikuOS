//! Stack minimo sopra SNP: ARP, IPv4, TCP client e HTTP/1.0 GET. Un socket alla volta.
//! Indirizzi di QEMU user networking: guest 10.0.2.15, host 10.0.2.2 (gateway).
use crate::net::{self, Nic};

pub const MY_IP: [u8; 4] = [10, 0, 2, 15];
pub const GW_IP: [u8; 4] = [10, 0, 2, 2];

const F_FIN: u8 = 0x01;
const F_SYN: u8 = 0x02;
const F_RST: u8 = 0x04;
const F_PSH: u8 = 0x08;
const F_ACK: u8 = 0x10;
const LPORT: u16 = 40000;

fn sum(mut acc: u32, d: &[u8]) -> u32 {
    let mut i = 0;
    while i + 1 < d.len() {
        acc += ((d[i] as u32) << 8) | d[i + 1] as u32;
        i += 2;
    }
    if i < d.len() {
        acc += (d[i] as u32) << 8;
    }
    acc
}

fn fold(mut acc: u32) -> u16 {
    while acc >> 16 != 0 {
        acc = (acc & 0xFFFF) + (acc >> 16);
    }
    !(acc as u16)
}

fn be16(d: &[u8], i: usize) -> u16 {
    ((d[i] as u16) << 8) | d[i + 1] as u16
}

fn be32(d: &[u8], i: usize) -> u32 {
    ((be16(d, i) as u32) << 16) | be16(d, i + 2) as u32
}

fn put16(d: &mut [u8], i: usize, v: u16) {
    d[i] = (v >> 8) as u8;
    d[i + 1] = v as u8;
}

fn put32(d: &mut [u8], i: usize, v: u32) {
    put16(d, i, (v >> 16) as u16);
    put16(d, i + 2, v as u16);
}

/// Frame ARP di 60 byte (minimo Ethernet): oper 1 = richiesta, 2 = risposta.
fn arp_frame(out: &mut [u8], mac: [u8; 6], oper: u16, dst_mac: [u8; 6], tha: [u8; 6], tpa: [u8; 4]) -> usize {
    out[0..6].copy_from_slice(&dst_mac);
    out[6..12].copy_from_slice(&mac);
    put16(out, 12, 0x0806);
    put16(out, 14, 1);
    put16(out, 16, 0x0800);
    out[18] = 6;
    out[19] = 4;
    put16(out, 20, oper);
    out[22..28].copy_from_slice(&mac);
    out[28..32].copy_from_slice(&MY_IP);
    out[32..38].copy_from_slice(&tha);
    out[38..42].copy_from_slice(&tpa);
    for b in &mut out[42..60] {
        *b = 0;
    }
    60
}

/// Risponde alle ARP per il nostro IP e impara il MAC del gateway.
fn handle_arp(nic: &Nic, f: &[u8], n: usize, gmac: &mut [u8; 6]) {
    if n < 42 || be16(f, 12) != 0x0806 {
        return;
    }
    let mut sha = [0u8; 6];
    sha.copy_from_slice(&f[22..28]);
    let mut spa = [0u8; 4];
    spa.copy_from_slice(&f[28..32]);
    let oper = be16(f, 20);
    if oper == 1 && f[38..42] == MY_IP {
        let mut out = [0u8; 60];
        let len = arp_frame(&mut out, nic.mac, 2, sha, sha, spa);
        net::send(nic, &out[..len]);
    } else if oper == 2 && spa == GW_IP {
        *gmac = sha;
    }
}

struct Conn {
    nic: Nic,
    gmac: [u8; 6],
    rport: u16,
    seq: u32,
    ack: u32,
}

/// Frame TCP verso il gateway: Ethernet + IPv4 + TCP con checksum.
fn tcp_frame(f: &mut [u8], c: &Conn, flags: u8, p: &[u8]) -> usize {
    let tl = 20 + p.len();
    f[0..6].copy_from_slice(&c.gmac);
    f[6..12].copy_from_slice(&c.nic.mac);
    put16(f, 12, 0x0800);
    f[14] = 0x45;
    f[15] = 0;
    put16(f, 16, (20 + tl) as u16);
    put16(f, 18, 0);
    put16(f, 20, 0x4000);
    f[22] = 64;
    f[23] = 6;
    put16(f, 24, 0);
    f[26..30].copy_from_slice(&MY_IP);
    f[30..34].copy_from_slice(&GW_IP);
    let ic = fold(sum(0, &f[14..34]));
    put16(f, 24, ic);
    put16(f, 34, LPORT);
    put16(f, 36, c.rport);
    put32(f, 38, c.seq);
    put32(f, 42, c.ack);
    f[46] = 0x50;
    f[47] = flags;
    put16(f, 48, 8192);
    put16(f, 50, 0);
    put16(f, 52, 0);
    f[54..54 + p.len()].copy_from_slice(p);
    let end = 54 + p.len();
    let mut acc = sum(0, &MY_IP);
    acc = sum(acc, &GW_IP);
    acc += 6 + tl as u32;
    acc = sum(acc, &f[34..end]);
    put16(f, 50, fold(acc));
    end
}

fn tcp_send(c: &Conn, flags: u8, p: &[u8]) {
    let mut f = [0u8; 1600];
    let n = tcp_frame(&mut f, c, flags, p).max(60); // minimo Ethernet, senza FCS
    net::send(&c.nic, &f[..n]);
}

/// Segmento TCP da GW verso la nostra porta: (flags, seq, inizio payload, fine payload).
fn parse_tcp(f: &[u8], n: usize, rport: u16) -> Option<(u8, u32, usize, usize)> {
    if n < 54 || be16(f, 12) != 0x0800 || f[14] != 0x45 || f[23] != 6 {
        return None;
    }
    if f[26..30] != GW_IP || be16(f, 34) != rport || be16(f, 36) != LPORT {
        return None;
    }
    let end = (14 + be16(f, 16) as usize).min(n);
    let start = 34 + ((f[46] >> 4) as usize) * 4;
    if start > end {
        return None;
    }
    Some((f[47], be32(f, 38), start, end))
}

/// HTTP/1.0 GET a GW_IP:`rport`. Scrive il corpo in `out`, ritorna la lunghezza.
pub fn http_get(nic: &Nic, rport: u16, path: &str, out: &mut [u8]) -> Result<usize, &'static str> {
    let mut f = [0u8; 1600];
    let mut gmac = [0u8; 6];
    let t0 = net::now_ms();
    let mut next = 0u64;
    // 1. MAC del gateway via ARP
    while gmac == [0u8; 6] {
        let now = net::now_ms();
        if now - t0 > 6000 {
            return Err("NESSUN GATEWAY");
        }
        if now >= next {
            let mut q = [0u8; 60];
            let l = arp_frame(&mut q, nic.mac, 1, [0xff; 6], [0; 6], GW_IP);
            net::send(nic, &q[..l]);
            next = now + 1000;
        }
        if let Some(n) = net::recv(nic, &mut f) {
            handle_arp(nic, &f, n, &mut gmac);
        }
    }
    // 2. Handshake TCP (SYN ritrasmesso ogni secondo)
    let mut c = Conn { nic: *nic, gmac, rport, seq: 1000, ack: 0 };
    let t1 = net::now_ms();
    next = 0;
    loop {
        let now = net::now_ms();
        if now - t1 > 8000 {
            return Err("NESSUNA RISPOSTA");
        }
        if now >= next {
            tcp_send(&c, F_SYN, &[]);
            next = now + 1000;
        }
        if let Some(n) = net::recv(nic, &mut f) {
            handle_arp(nic, &f, n, &mut gmac);
            c.gmac = gmac;
            if let Some((fl, seq, _, _)) = parse_tcp(&f, n, rport) {
                if fl & F_RST != 0 {
                    return Err("RIFIUTATO");
                }
                if fl & (F_SYN | F_ACK) == (F_SYN | F_ACK) {
                    c.ack = seq.wrapping_add(1);
                    break;
                }
            }
        }
    }
    c.seq = c.seq.wrapping_add(1);
    tcp_send(&c, F_ACK, &[]);
    // 3. Richiesta
    let mut req = [0u8; 256];
    let mut rn = 0;
    for part in ["GET ", path, " HTTP/1.0\r\nHost: 10.0.2.2\r\nConnection: close\r\n\r\n"] {
        let b = part.as_bytes();
        if rn + b.len() > req.len() {
            return Err("RICHIESTA TROPPO LUNGA");
        }
        req[rn..rn + b.len()].copy_from_slice(b);
        rn += b.len();
    }
    tcp_send(&c, F_PSH | F_ACK, &req[..rn]);
    let req_seq = c.seq;
    c.seq = c.seq.wrapping_add(rn as u32);
    let mut req_t = net::now_ms();
    let mut got_data = false;
    // 4. Risposta fino a FIN (o silenzio di 5 s)
    let mut stored = 0usize;
    let mut last = net::now_ms();
    loop {
        let now = net::now_ms();
        if now - last > 5000 {
            return if stored > 0 { Ok(stored) } else { Err("TIMEOUT") };
        }
        // Richiesta persa: la rimanda finché non arriva il primo dato.
        if !got_data && now - req_t > 1500 {
            let mut r = Conn { nic: c.nic, gmac: c.gmac, rport: c.rport, seq: req_seq, ack: c.ack };
            r.seq = req_seq;
            tcp_send(&r, F_PSH | F_ACK, &req[..rn]);
            req_t = now;
        }
        let n = match net::recv(nic, &mut f) {
            Some(n) => n,
            None => continue,
        };
        handle_arp(nic, &f, n, &mut gmac);
        let Some((fl, seq, s, e)) = parse_tcp(&f, n, rport) else { continue };
        if fl & F_RST != 0 {
            return Err("RESET");
        }
        let plen = e - s;
        if plen > 0 {
            got_data = true;
        }
        if seq != c.ack {
            if plen > 0 {
                tcp_send(&c, F_ACK, &[]);
            }
            continue;
        }
        let k = plen.min(out.len() - stored);
        out[stored..stored + k].copy_from_slice(&f[s..s + k]);
        stored += k;
        c.ack = c.ack.wrapping_add(plen as u32);
        last = now;
        if fl & F_FIN != 0 {
            c.ack = c.ack.wrapping_add(1);
            tcp_send(&c, F_ACK, &[]);
            tcp_send(&c, F_FIN | F_ACK, &[]);
            return Ok(stored);
        }
        if plen > 0 {
            tcp_send(&c, F_ACK, &[]);
        }
    }
}

/// Codice di stato e offset del corpo in una risposta HTTP.
pub fn split_http(r: &[u8]) -> (u16, usize) {
    let mut code = 0u16;
    if r.len() >= 12 {
        for i in 9..12 {
            code = code * 10 + (r[i].wrapping_sub(b'0') as u16);
        }
    }
    let mut off = r.len();
    for i in 0..r.len().saturating_sub(3) {
        if &r[i..i + 4] == b"\r\n\r\n" {
            off = i + 4;
            break;
        }
    }
    (code, off)
}
