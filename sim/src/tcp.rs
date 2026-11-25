//! TCP essenziale nel simulatore: handshake a tre vie con ritrasmissione del SYN, dati in ordine,
//! finestra, controllo di congestione (slow start, congestion avoidance, collasso a timeout),
//! RTO adattivo (Jacobson/Karels, in tick) con backoff, FIN. Sta sopra `ip.rs`.
//! Con buffer fuori ordine, SACK (fino a 3 blocchi) e ritrasmissione veloce a 3 ACK duplicati.
//! Non ancora: Nagle, finestra scalata, SACK nel mittente oltre il primo blocco.
//! Nessun hardware reale. Regole: docs/resource-microkernels.md, sezione 12.

use crate::ip::PROTO_TCP;
use crate::net::internet_checksum;
use std::collections::{BTreeMap, VecDeque};

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
    /// Blocchi SACK (inizio, fine) dei dati fuori ordine ricevuti. Al più 3.
    pub sack: Vec<(u32, u32)>,
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
        let blocks = &self.sack[..self.sack.len().min(3)];
        let mut opt = vec![];
        if !blocks.is_empty() {
            // opzione SACK (kind 5): NOP NOP, kind, lunghezza, blocchi. Lunghezza multipla di 4.
            opt.extend_from_slice(&[1, 1, 5, (2 + 8 * blocks.len()) as u8]);
            for (l, r) in blocks {
                opt.extend_from_slice(&l.to_be_bytes());
                opt.extend_from_slice(&r.to_be_bytes());
            }
        }
        let hlen = HDR + opt.len();
        let mut h = vec![0u8; hlen];
        h[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        h[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        h[4..8].copy_from_slice(&self.seq.to_be_bytes());
        h[8..12].copy_from_slice(&self.ack.to_be_bytes());
        h[12] = ((hlen / 4) as u8) << 4; // offset dati in parole di 32 bit
        h[13] = self.flags;
        h[14..16].copy_from_slice(&self.wnd.to_be_bytes());
        h[HDR..].copy_from_slice(&opt);
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
        let off = (raw[12] >> 4) as usize * 4;
        if off < HDR || off > raw.len() {
            return Err(TcpError::TooShort);
        }
        let mut sack = vec![];
        let mut i = HDR;
        while i < off {
            match raw[i] {
                0 => break,
                1 => i += 1,
                kind => {
                    if i + 1 >= off {
                        break;
                    }
                    let len = raw[i + 1] as usize;
                    if len < 2 || i + len > off {
                        break;
                    }
                    if kind == 5 {
                        for b in raw[i + 2..i + len].chunks_exact(8) {
                            let l = u32::from_be_bytes(b[0..4].try_into().unwrap());
                            let r = u32::from_be_bytes(b[4..8].try_into().unwrap());
                            sack.push((l, r));
                        }
                    }
                    i += len;
                }
            }
        }
        Ok(Segment {
            src_port: u16::from_be_bytes([raw[0], raw[1]]),
            dst_port: u16::from_be_bytes([raw[2], raw[3]]),
            seq: u32::from_be_bytes(raw[4..8].try_into().unwrap()),
            ack: u32::from_be_bytes(raw[8..12].try_into().unwrap()),
            flags: raw[13],
            wnd: u16::from_be_bytes([raw[14], raw[15]]),
            sack,
            payload: raw[off..].to_vec(),
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
    /// Confermato fuori ordine da un blocco SACK del peer.
    sacked: bool,
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
    /// Dati fuori ordine, per inizio di sequenza. Contano nella finestra.
    ooo: BTreeMap<u32, Vec<u8>>,
    /// ACK duplicati consecutivi.
    dupacks: u32,
    /// Ritrasmissione veloce pronta da inviare.
    fast_rexmit: Option<Segment>,
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
            ooo: BTreeMap::new(),
            dupacks: 0,
            fast_rexmit: None,
        }
    }

    pub fn listen(port: u16) -> Self {
        Self::blank(State::Listen, port, 0, 1000)
    }

    /// Apertura attiva: ritorna il SYN da inviare.
    pub fn connect(local: u16, remote: u16, iss: u32) -> (Self, Segment) {
        let mut c = Self::blank(State::SynSent, local, remote, iss);
        c.snd_nxt = iss.wrapping_add(1); // il SYN occupa un numero di sequenza
        let syn = Segment { src_port: local, dst_port: remote, seq: iss, ack: 0, flags: FLAG_SYN, wnd: c.window(), sack: vec![], payload: vec![] };
        c.hs = Some(syn.clone());
        (c, syn)
    }

    fn ooo_bytes(&self) -> usize {
        self.ooo.values().map(|v| v.len()).sum()
    }

    fn window(&self) -> u16 {
        let used = self.received.len() + self.ooo_bytes();
        (self.rcv_buf.saturating_sub(used)).min(u16::MAX as usize) as u16
    }

    /// Blocchi SACK: intervalli contigui dei dati fuori ordine, al più 3.
    fn sack_blocks(&self) -> Vec<(u32, u32)> {
        let mut out: Vec<(u32, u32)> = vec![];
        for (&k, v) in &self.ooo {
            let end = k.wrapping_add(v.len() as u32);
            match out.last_mut() {
                Some(last) if last.1 == k => last.1 = end,
                _ => out.push((k, end)),
            }
        }
        out.truncate(3);
        out
    }

    fn seg(&self, seq: u32, flags: u8, payload: Vec<u8>) -> Segment {
        Segment {
            src_port: self.local,
            dst_port: self.remote,
            seq,
            ack: self.rcv_nxt,
            flags,
            wnd: self.window(),
            sack: self.sack_blocks(),
            payload,
        }
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
                    self.on_ack_full(s);
                }
                let mut reply = None;
                if !s.payload.is_empty() {
                    self.on_data(s.seq, &s.payload);
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

    /// Riceve dati: in ordine vanno nel buffer e si svuota l'ooo; fuori ordine si tengono se entrano.
    fn on_data(&mut self, seq: u32, payload: &[u8]) {
        let d = seq.wrapping_sub(self.rcv_nxt) as i32;
        let held = self.received.len() + self.ooo_bytes();
        if d < 0 || payload.len() > self.rcv_buf.saturating_sub(held) {
            return; // già ricevuto, o non entra nel buffer: si conferma ciò che manca
        }
        if d > 0 {
            if (d as usize) < self.rcv_buf {
                self.ooo.entry(seq).or_insert_with(|| payload.to_vec());
            }
            return;
        }
        self.received.extend_from_slice(payload);
        self.rcv_nxt = self.rcv_nxt.wrapping_add(payload.len() as u32);
        // svuota i segmenti fuori ordine che ora sono in sequenza
        while let Some((&k, _)) = self.ooo.first_key_value() {
            if k.wrapping_sub(self.rcv_nxt) as i32 > 0 {
                break;
            }
            let v = self.ooo.remove(&k).unwrap();
            let skip = self.rcv_nxt.wrapping_sub(k) as usize;
            if skip < v.len() {
                self.received.extend_from_slice(&v[skip..]);
                self.rcv_nxt = self.rcv_nxt.wrapping_add((v.len() - skip) as u32);
            }
        }
    }

    /// Gestisce un ACK con eventuali blocchi SACK: conferma, rileva i duplicati e la ritrasmissione veloce.
    fn on_ack_full(&mut self, s: &Segment) {
        self.mark_sacked(&s.sack);
        if s.ack == self.snd_una && s.payload.is_empty() && !self.outstanding.is_empty() {
            self.dupacks += 1;
            if self.dupacks == 3 {
                // Ritrasmissione veloce: primo segmento non confermato e non SACKato. Niente timeout.
                let inflight = self.in_flight();
                self.ssthresh = (inflight / 2).max(2 * MSS);
                self.cwnd = self.ssthresh;
                if let Some(o) = self.outstanding.iter_mut().find(|o| !o.sacked) {
                    o.rexmit = true;
                    self.fast_rexmit = Some(o.seg.clone());
                }
            }
        } else {
            self.dupacks = 0;
            self.on_ack(s.ack);
        }
    }

    /// Segna come SACKati i segmenti interamente coperti da un blocco.
    fn mark_sacked(&mut self, blocks: &[(u32, u32)]) {
        for o in self.outstanding.iter_mut() {
            let l = o.seg.seq;
            let r = l.wrapping_add(o.seg.payload.len() as u32);
            if blocks.iter().any(|&(bl, br)| (l.wrapping_sub(bl) as i32) >= 0 && (br.wrapping_sub(r) as i32) >= 0) {
                o.sacked = true;
            }
        }
    }

    /// La ritrasmissione veloce pronta, se c'è. Va inviata subito.
    pub fn take_fast_rexmit(&mut self) -> Option<Segment> {
        self.fast_rexmit.take()
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
            self.outstanding.push_back(Out { seg: s.clone(), sent_at: self.now, rexmit: false, sacked: false });
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
            .filter(|o| !o.sacked)
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
