//! TCP essenziale nel simulatore: handshake a tre vie con ritrasmissione del SYN, dati in ordine,
//! finestra, controllo di congestione (slow start, congestion avoidance, collasso a timeout),
//! RTO adattivo (Jacobson/Karels, in tick) con backoff, FIN. Sta sopra `ip.rs`.
//! Non ancora: buffer fuori ordine, SACK, Nagle, ritrasmissione veloce.
//! Nessun hardware reale. Regole: docs/resource-microkernels.md, sezione 12.

use crate::ip::PROTO_TCP;
use crate::net::internet_checksum;
use std::collections::VecDeque;

pub const FLAG_FIN: u8 = 0x01;
pub const FLAG_SYN: u8 = 0x02;
pub const FLAG_ACK: u8 = 0x10;
const HDR: usize = 20;
/// RTO iniziale in tick, prima di avere una misura. Anche il timer fisso del SYN.
pub const RTO_TICKS: u32 = 3;
pub const RTO_MIN: u32 = 1;
pub const RTO_MAX: u32 = 64;
/// Dimensione massima di segmento, in byte. Dimensione di una finestra di congestione iniziale.
pub const MSS: usize = 128;
/// Buffer di ricezione di default, in byte. Profilo GBA: 256 (vedi docs, sezione 13).
pub const RCV_BUF: usize = 1024;
/// Tentativi di SYN o SYN-ACK prima di chiudere.
pub const HS_MAX_TRIES: u32 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq: u32,
    pub ack: u32,
    pub flags: u8,
    /// Finestra annunciata: byte che il mittente può ancora inviare.
    pub wnd: u16,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TcpError {
    TooShort,
    BadChecksum,
}

fn pseudo(src: [u8; 4], dst: [u8; 4], len: usize) -> Vec<u8> {
    let mut p = Vec::with_capacity(12);
    p.extend_from_slice(&src);
    p.extend_from_slice(&dst);
    p.push(0);
    p.push(PROTO_TCP);
    p.extend_from_slice(&(len as u16).to_be_bytes());
    p
}

impl Segment {
    /// Codifica con checksum su pseudo-intestazione (indirizzi IP di sorgente e destinazione).
    pub fn encode(&self, src_ip: [u8; 4], dst_ip: [u8; 4]) -> Vec<u8> {
        let mut h = vec![0u8; HDR];
        h[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        h[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        h[4..8].copy_from_slice(&self.seq.to_be_bytes());
        h[8..12].copy_from_slice(&self.ack.to_be_bytes());
        h[12] = 5 << 4; // offset dati 5 parole
        h[13] = self.flags;
        h[14..16].copy_from_slice(&self.wnd.to_be_bytes());
        h.extend_from_slice(&self.payload);
        let mut all = pseudo(src_ip, dst_ip, h.len());
        all.extend_from_slice(&h);
        let c = internet_checksum(&all);
        h[16..18].copy_from_slice(&c.to_be_bytes());
        h
    }

    pub fn parse(raw: &[u8], src_ip: [u8; 4], dst_ip: [u8; 4]) -> Result<Segment, TcpError> {
        if raw.len() < HDR {
            return Err(TcpError::TooShort);
        }
        let mut all = pseudo(src_ip, dst_ip, raw.len());
        all.extend_from_slice(raw);
        if internet_checksum(&all) != 0 {
            return Err(TcpError::BadChecksum);
        }
        Ok(Segment {
            src_port: u16::from_be_bytes([raw[0], raw[1]]),
            dst_port: u16::from_be_bytes([raw[2], raw[3]]),
            seq: u32::from_be_bytes(raw[4..8].try_into().unwrap()),
            ack: u32::from_be_bytes(raw[8..12].try_into().unwrap()),
            flags: raw[13],
            wnd: u16::from_be_bytes([raw[14], raw[15]]),
            payload: raw[HDR..].to_vec(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait,
    CloseWait,
}

/// Segmento in attesa di ACK, con l'istante di invio per la misura del RTT.
struct Out {
    seg: Segment,
    sent_at: u32,
    /// Ritrasmesso: non serve per misurare il RTT (regola di Karn).
    rexmit: bool,
}

/// Una connessione: stato, sequenze, finestre, timer, segmenti in attesa di ACK, dati ricevuti.
pub struct Conn {
    pub state: State,
    local: u16,
    remote: u16,
    /// Primo byte non ancora confermato dal peer.
    snd_una: u32,
    snd_nxt: u32,
    rcv_nxt: u32,
    /// Ultima finestra annunciata dal peer.
    peer_wnd: u16,
    /// Dimensione del buffer di ricezione (byte).
    pub rcv_buf: usize,
    outstanding: VecDeque<Out>,
    /// Orologio in tick.
    now: u32,
    /// Tick dall'ultimo progresso: serve al timer di ritrasmissione.
    idle: u32,
    srtt: Option<u32>,
    rttvar: u32,
    rto: u32,
    cwnd: usize,
    ssthresh: usize,
    /// SYN o SYN-ACK in attesa di risposta, con i suoi tentativi.
    hs: Option<Segment>,
    hs_idle: u32,
    hs_tries: u32,
    pub received: Vec<u8>,
}

impl Conn {
    fn blank(state: State, local: u16, remote: u16, iss: u32) -> Self {
        Self {
            state,
            local,
            remote,
            snd_una: iss,
            snd_nxt: iss,
            rcv_nxt: 0,
            peer_wnd: RCV_BUF as u16,
            rcv_buf: RCV_BUF,
            outstanding: VecDeque::new(),
            now: 0,
            idle: 0,
            srtt: None,
            rttvar: 0,
            rto: RTO_TICKS,
            cwnd: MSS,
            ssthresh: RCV_BUF,
            hs: None,
            hs_idle: 0,
            hs_tries: 0,
            received: vec![],
        }
    }

    pub fn listen(port: u16) -> Self {
        Self::blank(State::Listen, port, 0, 1000)
    }

    /// Apertura attiva: ritorna il SYN da inviare.
    pub fn connect(local: u16, remote: u16, iss: u32) -> (Self, Segment) {
        let mut c = Self::blank(State::SynSent, local, remote, iss);
        c.snd_nxt = iss.wrapping_add(1); // il SYN occupa un numero di sequenza
        let syn = Segment { src_port: local, dst_port: remote, seq: iss, ack: 0, flags: FLAG_SYN, wnd: c.window(), payload: vec![] };
        c.hs = Some(syn.clone());
        (c, syn)
    }

    fn window(&self) -> u16 {
        (self.rcv_buf.saturating_sub(self.received.len())).min(u16::MAX as usize) as u16
    }

    fn seg(&self, seq: u32, flags: u8, payload: Vec<u8>) -> Segment {
        Segment { src_port: self.local, dst_port: self.remote, seq, ack: self.rcv_nxt, flags, wnd: self.window(), payload }
    }

    /// Gestisce un segmento in arrivo. Ritorna la risposta da inviare, se c'è.
    pub fn on_segment(&mut self, s: &Segment) -> Option<Segment> {
        if s.dst_port != self.local {
            return None;
        }
        self.peer_wnd = s.wnd;
        match self.state {
            State::Listen if s.flags & FLAG_SYN != 0 => {
                self.remote = s.src_port;
                self.rcv_nxt = s.seq.wrapping_add(1);
                let synack = self.seg(self.snd_nxt, FLAG_SYN | FLAG_ACK, vec![]);
                self.snd_nxt = self.snd_nxt.wrapping_add(1); // il SYN-ACK occupa un numero
                self.hs = Some(synack.clone());
                self.state = State::SynReceived;
                Some(synack)
            }
            // SYN ripetuto: il SYN-ACK si è perso, lo rimandiamo.
            State::SynReceived if s.flags & FLAG_SYN != 0 => self.hs.clone(),
            State::SynSent if s.flags & (FLAG_SYN | FLAG_ACK) == (FLAG_SYN | FLAG_ACK) => {
                self.rcv_nxt = s.seq.wrapping_add(1);
                self.snd_una = self.snd_nxt;
                self.hs = None;
                self.state = State::Established;
                Some(self.seg(self.snd_nxt, FLAG_ACK, vec![]))
            }
            State::SynReceived if s.flags & FLAG_ACK != 0 => {
                self.snd_una = self.snd_nxt;
                self.hs = None;
                self.state = State::Established;
                None
            }
            State::Established | State::CloseWait | State::FinWait => {
                // SYN-ACK ripetuto dopo il nostro ACK perso: rispondiamo con un ACK.
                if s.flags & FLAG_SYN != 0 {
                    return Some(self.seg(self.snd_nxt, FLAG_ACK, vec![]));
                }
                if s.flags & FLAG_ACK != 0 {
                    self.on_ack(s.ack);
                }
                let mut reply = None;
                if !s.payload.is_empty() {
                    // Solo in ordine e se entra nel buffer; il resto è scartato, con ACK di ciò che manca.
                    if s.seq == self.rcv_nxt && s.payload.len() <= self.window() as usize {
                        self.received.extend_from_slice(&s.payload);
                        self.rcv_nxt = self.rcv_nxt.wrapping_add(s.payload.len() as u32);
                    }
                    reply = Some(self.seg(self.snd_nxt, FLAG_ACK, vec![]));
                }
                if s.flags & FLAG_FIN != 0 {
                    self.rcv_nxt = self.rcv_nxt.wrapping_add(1);
                    self.state = State::CloseWait;
                    reply = Some(self.seg(self.snd_nxt, FLAG_ACK, vec![]));
                }
                reply
            }
            _ => None,
        }
    }

    /// Conferma i segmenti fino ad `ack`: misura il RTT, fa crescere la finestra di congestione.
    fn on_ack(&mut self, ack: u32) {
        let newly = ack.wrapping_sub(self.snd_una);
        let in_flight = self.snd_nxt.wrapping_sub(self.snd_una);
        if newly == 0 || newly > in_flight {
            return;
        }
        self.snd_una = ack;
        let mut sample = None;
        loop {
            let end = match self.outstanding.front() {
                Some(f) => f.seg.seq.wrapping_add(f.seg.payload.len() as u32),
                None => break,
            };
            if (ack.wrapping_sub(end) as i32) < 0 {
                break;
            }
            let o = self.outstanding.pop_front().unwrap();
            if !o.rexmit {
                sample = Some(self.now.wrapping_sub(o.sent_at).max(1));
            }
        }
        // Congestione: slow start fino a ssthresh, poi crescita lineare (~1 MSS per RTT).
        let n = newly as usize;
        if self.cwnd < self.ssthresh {
            self.cwnd += n.min(MSS);
        } else {
            self.cwnd += (MSS * MSS / self.cwnd).max(1);
        }
        if let Some(r) = sample {
            self.rtt_update(r);
        }
        self.idle = 0;
    }

    /// Jacobson/Karels: stima di SRTT e RTTVAR, RTO = SRTT + 4·RTTVAR, limitato.
    fn rtt_update(&mut self, r: u32) {
        match self.srtt {
            None => {
                self.srtt = Some(r);
                self.rttvar = r / 2;
            }
            Some(s) => {
                let err = if r > s { r - s } else { s - r };
                self.rttvar = (3 * self.rttvar + err) / 4;
                self.srtt = Some((7 * s + r) / 8);
            }
        }
        let s = self.srtt.unwrap_or(r);
        self.rto = (s + 4 * self.rttvar).clamp(RTO_MIN, RTO_MAX);
    }

    /// Invia dati in stato Established, nei limiti di finestra del peer e di congestione.
    /// Ritorna `None` se non c'è stato o se i dati non entrano.
    pub fn send(&mut self, data: &[u8]) -> Option<Segment> {
        if self.state != State::Established {
            return None;
        }
        let limit = (self.peer_wnd as usize).min(self.cwnd);
        if self.in_flight() + data.len() > limit {
            return None;
        }
        let s = self.seg(self.snd_nxt, FLAG_ACK, data.to_vec());
        self.snd_nxt = self.snd_nxt.wrapping_add(data.len() as u32);
        if !data.is_empty() {
            if self.outstanding.is_empty() {
                self.idle = 0;
            }
            self.outstanding.push_back(Out { seg: s.clone(), sent_at: self.now, rexmit: false });
        }
        Some(s)
    }

    /// Un tick del timer. Ritrasmette il SYN/SYN-ACK in sospeso, oppure, allo scadere del RTO,
    /// tutti i segmenti non confermati (go-back-N) con collasso della congestione e backoff.
    pub fn tick(&mut self) -> Vec<Segment> {
        self.now = self.now.wrapping_add(1);
        if let Some(h) = self.hs.clone() {
            self.hs_idle += 1;
            if self.hs_idle < RTO_TICKS {
                return vec![];
            }
            self.hs_idle = 0;
            self.hs_tries += 1;
            if self.hs_tries > HS_MAX_TRIES {
                self.hs = None;
                self.state = State::Closed;
                return vec![];
            }
            return vec![h];
        }
        if self.outstanding.is_empty() {
            self.idle = 0;
            return vec![];
        }
        self.idle += 1;
        if self.idle < self.rto {
            return vec![];
        }
        // Timeout: ssthresh a metà del volume in volo, finestra di congestione a 1 MSS, RTO raddoppiato.
        self.ssthresh = (self.in_flight() / 2).max(2 * MSS);
        self.cwnd = MSS;
        self.rto = (self.rto * 2).min(RTO_MAX);
        self.idle = 0;
        self.outstanding
            .iter_mut()
            .map(|o| {
                o.rexmit = true;
                o.seg.clone()
            })
            .collect()
    }

    /// Finestra di congestione corrente, in byte.
    pub fn cwnd(&self) -> usize {
        self.cwnd
    }

    /// Timeout di ritrasmissione corrente, in tick.
    pub fn rto(&self) -> u32 {
        self.rto
    }

    /// L'applicazione legge i dati ricevuti. Se la finestra si apre, ritorna un ACK per il peer.
    pub fn read(&mut self) -> (Vec<u8>, Option<Segment>) {
        let data = std::mem::take(&mut self.received);
        let ack = if data.is_empty() || self.state == State::Closed {
            None
        } else {
            Some(self.seg(self.snd_nxt, FLAG_ACK, vec![]))
        };
        (data, ack)
    }

    /// Chiude l'invio con FIN.
    pub fn close(&mut self) -> Option<Segment> {
        if self.state != State::Established {
            return None;
        }
        let s = self.seg(self.snd_nxt, FLAG_FIN | FLAG_ACK, vec![]);
        self.snd_nxt = self.snd_nxt.wrapping_add(1);
        self.state = State::FinWait;
        Some(s)
    }

    /// Dati inviati e non ancora confermati, in byte.
    pub fn in_flight(&self) -> usize {
        self.snd_nxt.wrapping_sub(self.snd_una) as usize
    }
}
