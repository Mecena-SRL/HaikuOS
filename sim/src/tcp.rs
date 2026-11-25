//! TCP essenziale nel simulatore: handshake a tre vie, dati in ordine, FIN.
//! Niente ritrasmissione, finestra o controllo di congestione: da fare. Sta sopra `ip.rs`.
//! Nessun hardware reale. Regole: docs/resource-microkernels.md, sezione 4.1.

use crate::ip::PROTO_TCP;
use crate::net::internet_checksum;

pub const FLAG_FIN: u8 = 0x01;
pub const FLAG_SYN: u8 = 0x02;
pub const FLAG_ACK: u8 = 0x10;
const HDR: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq: u32,
    pub ack: u32,
    pub flags: u8,
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

/// Una connessione: stato, numeri di sequenza, dati ricevuti in ordine.
pub struct Conn {
    pub state: State,
    local: u16,
    remote: u16,
    snd_nxt: u32,
    rcv_nxt: u32,
    pub received: Vec<u8>,
}

impl Conn {
    pub fn listen(port: u16) -> Self {
        Self { state: State::Listen, local: port, remote: 0, snd_nxt: 1000, rcv_nxt: 0, received: vec![] }
    }

    /// Apertura attiva: ritorna il SYN da inviare.
    pub fn connect(local: u16, remote: u16, iss: u32) -> (Self, Segment) {
        let c = Self { state: State::SynSent, local, remote, snd_nxt: iss + 1, rcv_nxt: 0, received: vec![] };
        let syn = Segment { src_port: local, dst_port: remote, seq: iss, ack: 0, flags: FLAG_SYN, payload: vec![] };
        (c, syn)
    }

    fn seg(&self, seq: u32, flags: u8, payload: Vec<u8>) -> Segment {
        Segment { src_port: self.local, dst_port: self.remote, seq, ack: self.rcv_nxt, flags, payload }
    }

    /// Gestisce un segmento in arrivo. Ritorna la risposta da inviare, se c'è.
    pub fn on_segment(&mut self, s: &Segment) -> Option<Segment> {
        if s.dst_port != self.local {
            return None;
        }
        match self.state {
            State::Listen if s.flags & FLAG_SYN != 0 => {
                self.remote = s.src_port;
                self.rcv_nxt = s.seq.wrapping_add(1);
                self.state = State::SynReceived;
                Some(self.seg(self.snd_nxt, FLAG_SYN | FLAG_ACK, vec![]))
            }
            State::SynSent if s.flags & (FLAG_SYN | FLAG_ACK) == (FLAG_SYN | FLAG_ACK) => {
                self.rcv_nxt = s.seq.wrapping_add(1);
                self.state = State::Established;
                Some(self.seg(self.snd_nxt, FLAG_ACK, vec![]))
            }
            State::SynReceived if s.flags & FLAG_ACK != 0 => {
                self.state = State::Established;
                None
            }
            State::Established | State::CloseWait | State::FinWait => {
                let mut reply = None;
                if !s.payload.is_empty() && s.seq == self.rcv_nxt {
                    self.received.extend_from_slice(&s.payload);
                    self.rcv_nxt = self.rcv_nxt.wrapping_add(s.payload.len() as u32);
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

    /// Invia dati in stato Established. Ritorna il segmento da inviare.
    pub fn send(&mut self, data: &[u8]) -> Option<Segment> {
        if self.state != State::Established {
            return None;
        }
        let s = self.seg(self.snd_nxt, FLAG_ACK, data.to_vec());
        self.snd_nxt = self.snd_nxt.wrapping_add(data.len() as u32);
        Some(s)
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
}
