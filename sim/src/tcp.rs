//! TCP essenziale nel simulatore: handshake a tre vie, dati in ordine, finestra di base,
//! ritrasmissione a timeout (go-back-N, RTO in tick), FIN. Sta sopra `ip.rs`.
//! Non ancora: controllo di congestione, buffer fuori ordine, RTO adattivo.
//! Nessun hardware reale. Regole: docs/resource-microkernels.md, sezione 12.

use crate::ip::PROTO_TCP;
use crate::net::internet_checksum;
use std::collections::VecDeque;

pub const FLAG_FIN: u8 = 0x01;
pub const FLAG_SYN: u8 = 0x02;
pub const FLAG_ACK: u8 = 0x10;
const HDR: usize = 20;
/// Tick senza ACK prima di ritrasmettere.
pub const RTO_TICKS: u32 = 3;
/// Buffer di ricezione di default, in byte: la finestra annunciata è il buffer libero.
pub const RCV_BUF: usize = 1024;

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

/// Una connessione: stato, sequenze, finestra, segmenti in attesa di ACK, dati ricevuti.
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
    /// Segmenti inviati e non ancora confermati, in ordine di sequenza.
    outstanding: VecDeque<Segment>,
    /// Tick dall'ultimo invio o dall'ultimo ACK ricevuto.
    idle: u32,
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
            idle: 0,
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
                self.state = State::SynReceived;
                Some(synack)
            }
            State::SynSent if s.flags & (FLAG_SYN | FLAG_ACK) == (FLAG_SYN | FLAG_ACK) => {
                self.rcv_nxt = s.seq.wrapping_add(1);
                self.snd_una = self.snd_nxt;
                self.state = State::Established;
                Some(self.seg(self.snd_nxt, FLAG_ACK, vec![]))
            }
            State::SynReceived if s.flags & FLAG_ACK != 0 => {
                self.snd_una = self.snd_nxt;
                self.state = State::Established;
                None
            }
            State::Established | State::CloseWait | State::FinWait => {
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

    /// Conferma i segmenti fino ad `ack` e riporta il timer a zero.
    fn on_ack(&mut self, ack: u32) {
        let newly = ack.wrapping_sub(self.snd_una);
        let in_flight = self.snd_nxt.wrapping_sub(self.snd_una);
        if newly == 0 || newly > in_flight {
            return;
        }
        self.snd_una = ack;
        while let Some(front) = self.outstanding.front() {
            let end = front.seq.wrapping_add(front.payload.len() as u32);
            if ack.wrapping_sub(end) as i32 >= 0 {
                self.outstanding.pop_front();
            } else {
                break;
            }
        }
        self.idle = 0;
    }

    /// Invia dati in stato Established, nei limiti della finestra del peer.
    /// Ritorna `None` se non c'è stato o se i dati non entrano nella finestra.
    pub fn send(&mut self, data: &[u8]) -> Option<Segment> {
        if self.state != State::Established {
            return None;
        }
        let in_flight = self.snd_nxt.wrapping_sub(self.snd_una) as usize;
        if in_flight + data.len() > self.peer_wnd as usize {
            return None;
        }
        let s = self.seg(self.snd_nxt, FLAG_ACK, data.to_vec());
        self.snd_nxt = self.snd_nxt.wrapping_add(data.len() as u32);
        if !data.is_empty() {
            self.outstanding.push_back(s.clone());
        }
        self.idle = 0;
        Some(s)
    }

    /// Un tick del timer. Se ci sono segmenti senza ACK da troppo tempo, li rimanda tutti (go-back-N).
    pub fn tick(&mut self) -> Vec<Segment> {
        if self.outstanding.is_empty() {
            return vec![];
        }
        self.idle += 1;
        if self.idle < RTO_TICKS {
            return vec![];
        }
        self.idle = 0;
        self.outstanding.iter().cloned().collect()
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
