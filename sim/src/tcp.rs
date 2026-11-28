//! TCP essenziale nel simulatore, senza allocazione (no_std: solo `core`).
//! Buffer fissi: RX e TX di `ConnN<RX>`, slot fissi per segmenti in attesa di ACK e per dati fuori ordine.
//! Handshake con ritrasmissione del SYN, dati in ordine, finestra, congestione (slow start,
//! congestion avoidance, collasso a timeout), RTO adattivo con backoff, SACK (3 blocchi),
//! ritrasmissione veloce a 3 ACK duplicati, FIN. Sta sopra `ip.rs`. Nessun hardware reale.
//! Profilo GBA: `ConnN<256>`. Simulatore host: `Conn` = `ConnN<1024>`.

use crate::ip::PROTO_TCP;

pub const FLAG_FIN: u8 = 0x01;
pub const FLAG_SYN: u8 = 0x02;
pub const FLAG_ACK: u8 = 0x10;
const HDR: usize = 20;
/// RTO iniziale in tick, prima di avere una misura. Anche il timer fisso del SYN.
pub const RTO_TICKS: u32 = 3;
pub const RTO_MIN: u32 = 1;
pub const RTO_MAX: u32 = 64;
/// Dimensione massima di segmento in byte.
pub const MSS: usize = 128;
/// Buffer di ricezione del simulatore host, in byte.
pub const RCV_BUF: usize = 1024;
/// Tentativi di SYN o SYN-ACK prima di chiudere.
pub const HS_MAX_TRIES: u32 = 5;
/// Segmenti in attesa di ACK per connessione.
pub const OUT_SLOTS: usize = 8;
/// Segmenti fuori ordine tenuti per connessione.
pub const OOO_SLOTS: usize = 4;
/// Blocchi SACK per segmento.
pub const MAX_SACK: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TcpError {
    TooShort,
    TooLong,
    BadChecksum,
    NoSpace,
}

/// Somma a 16 bit con riporto, senza allocazione. Ogni parte deve iniziare a indirizzo pari.
fn sum_words(mut acc: u32, data: &[u8]) -> u32 {
    let mut ch = data.chunks_exact(2);
    for c in &mut ch {
        acc += u16::from_be_bytes([c[0], c[1]]) as u32;
    }
    if let [l] = ch.remainder() {
        acc += (*l as u32) << 8;
    }
    acc
}

fn fold(mut s: u32) -> u16 {
    while s >> 16 != 0 {
        s = (s & 0xffff) + (s >> 16);
    }
    s as u16
}

/// Somma della pseudo-intestazione IPv4 per TCP.
fn pseudo_sum(src: [u8; 4], dst: [u8; 4], len: usize) -> u32 {
    let mut p = [0u8; 12];
    p[0..4].copy_from_slice(&src);
    p[4..8].copy_from_slice(&dst);
    p[9] = PROTO_TCP;
    p[10..12].copy_from_slice(&(len as u16).to_be_bytes());
    sum_words(0, &p)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segment {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq: u32,
    pub ack: u32,
    pub flags: u8,
    /// Finestra annunciata: byte che il mittente può ancora inviare.
    pub wnd: u16,
    sack: [(u32, u32); MAX_SACK],
    nsack: u8,
    payload: [u8; MSS],
    plen: u8,
}

impl Segment {
    pub fn new(src_port: u16, dst_port: u16, seq: u32, ack: u32, flags: u8, wnd: u16) -> Self {
        Self {
            src_port,
            dst_port,
            seq,
            ack,
            flags,
            wnd,
            sack: [(0, 0); MAX_SACK],
            nsack: 0,
            payload: [0; MSS],
            plen: 0,
        }
    }

    /// Imposta il payload. Oltre `MSS` il dato viene troncato: il chiamante spezza prima.
    pub fn with_data(mut self, data: &[u8]) -> Self {
        let n = data.len().min(MSS);
        self.payload[..n].copy_from_slice(&data[..n]);
        self.plen = n as u8;
        self
    }

    pub fn data(&self) -> &[u8] {
        &self.payload[..self.plen as usize]
    }

    pub fn sacks(&self) -> &[(u32, u32)] {
        &self.sack[..self.nsack as usize]
    }

    /// Imposta fino a `MAX_SACK` blocchi SACK.
    pub fn with_sacks(mut self, blocks: &[(u32, u32)]) -> Self {
        let n = blocks.len().min(MAX_SACK);
        self.sack[..n].copy_from_slice(&blocks[..n]);
        self.nsack = n as u8;
        self
    }

    /// Scrive il segmento in `out` con checksum su pseudo-intestazione. Ritorna la lunghezza.
    pub fn encode(&self, src_ip: [u8; 4], dst_ip: [u8; 4], out: &mut [u8]) -> Result<usize, TcpError> {
        let n = self.nsack as usize;
        let optlen = if n > 0 { 4 + 8 * n } else { 0 };
        let hlen = HDR + optlen;
        let total = hlen + self.plen as usize;
        if out.len() < total {
            return Err(TcpError::NoSpace);
        }
        let h = &mut out[..total];
        h.fill(0);
        h[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        h[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        h[4..8].copy_from_slice(&self.seq.to_be_bytes());
        h[8..12].copy_from_slice(&self.ack.to_be_bytes());
        h[12] = ((hlen / 4) as u8) << 4;
        h[13] = self.flags;
        h[14..16].copy_from_slice(&self.wnd.to_be_bytes());
        if n > 0 {
            // opzione SACK (kind 5): NOP NOP, kind, lunghezza, blocchi
            h[HDR..HDR + 4].copy_from_slice(&[1, 1, 5, (2 + 8 * n) as u8]);
            for (i, (l, r)) in self.sacks().iter().enumerate() {
                let o = HDR + 4 + 8 * i;
                h[o..o + 4].copy_from_slice(&l.to_be_bytes());
                h[o + 4..o + 8].copy_from_slice(&r.to_be_bytes());
            }
        }
        h[hlen..total].copy_from_slice(self.data());
        let acc = pseudo_sum(src_ip, dst_ip, total) + sum_words(0, h);
        h[16..18].copy_from_slice(&(!fold(acc)).to_be_bytes());
        Ok(total)
    }

    /// Legge un segmento e verifica il checksum. Il payload è copiato nel segmento.
    pub fn parse(raw: &[u8], src_ip: [u8; 4], dst_ip: [u8; 4]) -> Result<Segment, TcpError> {
        if raw.len() < HDR {
            return Err(TcpError::TooShort);
        }
        if fold(pseudo_sum(src_ip, dst_ip, raw.len()) + sum_words(0, raw)) != 0xffff {
            return Err(TcpError::BadChecksum);
        }
        let off = (raw[12] >> 4) as usize * 4;
        if off < HDR || off > raw.len() {
            return Err(TcpError::TooShort);
        }
        let seg = Segment::new(
            u16::from_be_bytes([raw[0], raw[1]]),
            u16::from_be_bytes([raw[2], raw[3]]),
            u32::from_be_bytes(raw[4..8].try_into().unwrap()),
            u32::from_be_bytes(raw[8..12].try_into().unwrap()),
            raw[13],
            u16::from_be_bytes([raw[14], raw[15]]),
        );
        let mut blocks = [(0u32, 0u32); MAX_SACK];
        let mut nb = 0;
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
                            if nb < MAX_SACK {
                                blocks[nb] = (
                                    u32::from_be_bytes(b[0..4].try_into().unwrap()),
                                    u32::from_be_bytes(b[4..8].try_into().unwrap()),
                                );
                                nb += 1;
                            }
                        }
                    }
                    i += len;
                }
            }
        }
        let body = &raw[off..];
        if body.len() > MSS {
            return Err(TcpError::TooLong);
        }
        Ok(seg.with_sacks(&blocks[..nb]).with_data(body))
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

/// Metadati di un segmento in attesa di ACK. I dati stanno nel buffer TX.
#[derive(Clone, Copy, Debug)]
struct Meta {
    seq: u32,
    len: u8,
    sent_at: u32,
    /// Ritrasmesso: non serve per misurare il RTT (regola di Karn).
    rexmit: bool,
    /// Confermato fuori ordine da un blocco SACK del peer.
    sacked: bool,
}

/// Dato fuori ordine in attesa del buco.
#[derive(Clone, Copy)]
struct Ooo {
    seq: u32,
    len: u8,
    data: [u8; MSS],
}

/// Connessione con buffer RX e TX di `RX` byte ciascuno. Tutto in memoria fissa.
pub struct ConnN<const RX: usize> {
    pub state: State,
    local: u16,
    remote: u16,
    /// Primo byte non ancora confermato dal peer.
    snd_una: u32,
    snd_nxt: u32,
    rcv_nxt: u32,
    /// Ultima finestra annunciata dal peer.
    peer_wnd: u16,
    /// Buffer di ricezione annunciato (byte), al più `RX`.
    pub rcv_buf: usize,
    /// Byte inviati e non confermati, dall'inizio (`snd_una`).
    tx: [u8; RX],
    tx_len: usize,
    meta: [Meta; OUT_SLOTS],
    nmeta: usize,
    /// Dati ricevuti in ordine e non ancora letti.
    rx: [u8; RX],
    rx_len: usize,
    ooo: [Ooo; OOO_SLOTS],
    nooo: usize,
    /// Orologio in tick.
    now: u32,
    /// Tick dall'ultimo progresso: serve al timer di ritrasmissione.
    idle: u32,
    srtt: Option<u32>,
    rttvar: u32,
    rto: u32,
    cwnd: usize,
    ssthresh: usize,
    /// SYN o SYN-ACK in attesa di risposta.
    hs: Option<Segment>,
    hs_idle: u32,
    hs_tries: u32,
    dupacks: u32,
    fast_rexmit: Option<Segment>,
    /// Ritrasmissione da timeout in corso: prossimo indice da rimandare.
    rexmit_active: bool,
    rexmit_next: usize,
}

/// Profilo di simulazione host.
pub type Conn = ConnN<RCV_BUF>;

impl<const RX: usize> ConnN<RX> {
    fn blank(state: State, local: u16, remote: u16, iss: u32) -> Self {
        Self {
            state,
            local,
            remote,
            snd_una: iss,
            snd_nxt: iss,
            rcv_nxt: 0,
            peer_wnd: RX.min(u16::MAX as usize) as u16,
            rcv_buf: RX,
            tx: [0; RX],
            tx_len: 0,
            meta: [Meta { seq: 0, len: 0, sent_at: 0, rexmit: false, sacked: false }; OUT_SLOTS],
            nmeta: 0,
            rx: [0; RX],
            rx_len: 0,
            ooo: [Ooo { seq: 0, len: 0, data: [0; MSS] }; OOO_SLOTS],
            nooo: 0,
            now: 0,
            idle: 0,
            srtt: None,
            rttvar: 0,
            rto: RTO_TICKS,
            cwnd: MSS,
            ssthresh: RX,
            hs: None,
            hs_idle: 0,
            hs_tries: 0,
            dupacks: 0,
            fast_rexmit: None,
            rexmit_active: false,
            rexmit_next: 0,
        }
    }

    pub fn listen(port: u16) -> Self {
        Self::blank(State::Listen, port, 0, 1000)
    }

    /// Apertura attiva: ritorna il SYN da inviare.
    pub fn connect(local: u16, remote: u16, iss: u32) -> (Self, Segment) {
        let mut c = Self::blank(State::SynSent, local, remote, iss);
        c.snd_nxt = iss.wrapping_add(1); // il SYN occupa un numero di sequenza
        let syn = Segment::new(local, remote, iss, 0, FLAG_SYN, c.window());
        c.hs = Some(syn);
        (c, syn)
    }

    fn ooo_bytes(&self) -> usize {
        self.ooo[..self.nooo].iter().map(|e| e.len as usize).sum()
    }

    /// Byte liberi nel buffer di ricezione: è la finestra annunciata.
    fn window(&self) -> u16 {
        let cap = self.rcv_buf.min(RX);
        let used = self.rx_len + self.ooo_bytes();
        (cap.saturating_sub(used)).min(u16::MAX as usize) as u16
    }

    /// Blocchi SACK: intervalli contigui dei dati fuori ordine, ordinati, al più 3.
    fn sack_blocks(&self) -> ([(u32, u32); MAX_SACK], usize) {
        let mut ranges = [(0u32, 0u32); OOO_SLOTS];
        for (i, e) in self.ooo[..self.nooo].iter().enumerate() {
            ranges[i] = (e.seq, e.seq.wrapping_add(e.len as u32));
        }
        let r = &mut ranges[..self.nooo];
        // ordinamento per inserimento: pochi elementi
        for i in 1..r.len() {
            let mut j = i;
            while j > 0 && (r[j - 1].0.wrapping_sub(r[j].0) as i32) > 0 {
                r.swap(j - 1, j);
                j -= 1;
            }
        }
        let mut out = [(0u32, 0u32); MAX_SACK];
        let mut n = 0;
        for &(l, rr) in r.iter() {
            if n > 0 && out[n - 1].1 == l {
                out[n - 1].1 = rr;
            } else if n < MAX_SACK {
                out[n] = (l, rr);
                n += 1;
            }
        }
        (out, n)
    }

    fn seg(&self, seq: u32, flags: u8) -> Segment {
        let (blocks, n) = self.sack_blocks();
        Segment::new(self.local, self.remote, seq, self.rcv_nxt, flags, self.window()).with_sacks(&blocks[..n])
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
                let synack = self.seg(self.snd_nxt, FLAG_SYN | FLAG_ACK);
                self.snd_nxt = self.snd_nxt.wrapping_add(1); // il SYN-ACK occupa un numero
                self.hs = Some(synack);
                self.state = State::SynReceived;
                Some(synack)
            }
            // SYN ripetuto: il SYN-ACK si è perso, lo rimandiamo.
            State::SynReceived if s.flags & FLAG_SYN != 0 => self.hs,
            State::SynSent if s.flags & (FLAG_SYN | FLAG_ACK) == (FLAG_SYN | FLAG_ACK) => {
                self.rcv_nxt = s.seq.wrapping_add(1);
                self.snd_una = self.snd_nxt;
                self.hs = None;
                self.state = State::Established;
                Some(self.seg(self.snd_nxt, FLAG_ACK))
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
                    return Some(self.seg(self.snd_nxt, FLAG_ACK));
                }
                if s.flags & FLAG_ACK != 0 {
                    self.on_ack_full(s);
                }
                let mut reply = None;
                if s.plen > 0 {
                    self.on_data(s.seq, s.data());
                    reply = Some(self.seg(self.snd_nxt, FLAG_ACK));
                }
                if s.flags & FLAG_FIN != 0 {
                    self.rcv_nxt = self.rcv_nxt.wrapping_add(1);
                    self.state = State::CloseWait;
                    reply = Some(self.seg(self.snd_nxt, FLAG_ACK));
                }
                reply
            }
            _ => None,
        }
    }

    /// Riceve dati: in ordine vanno nel buffer e si svuota il fuori ordine; il resto si tiene se entra.
    fn on_data(&mut self, seq: u32, data: &[u8]) {
        let d = seq.wrapping_sub(self.rcv_nxt) as i32;
        if d < 0 || data.len() > self.window() as usize {
            return; // già ricevuto, o non entra: si conferma ciò che manca
        }
        if d > 0 {
            let dup = self.ooo[..self.nooo].iter().any(|e| e.seq == seq);
            if !dup && self.nooo < OOO_SLOTS {
                let mut e = Ooo { seq, len: data.len() as u8, data: [0; MSS] };
                e.data[..data.len()].copy_from_slice(data);
                self.ooo[self.nooo] = e;
                self.nooo += 1;
            }
            return;
        }
        self.rx[self.rx_len..self.rx_len + data.len()].copy_from_slice(data);
        self.rx_len += data.len();
        self.rcv_nxt = self.rcv_nxt.wrapping_add(data.len() as u32);
        self.drain_ooo();
    }

    /// Sposta nel buffer di ricezione i dati fuori ordine che ora sono in sequenza.
    fn drain_ooo(&mut self) {
        loop {
            let pos = self.ooo[..self.nooo]
                .iter()
                .position(|e| (e.seq.wrapping_sub(self.rcv_nxt) as i32) <= 0);
            let Some(i) = pos else { break };
            let e = self.ooo[i];
            self.nooo -= 1;
            self.ooo[i] = self.ooo[self.nooo];
            let skip = self.rcv_nxt.wrapping_sub(e.seq) as usize;
            let len = e.len as usize;
            if skip < len {
                let part = &e.data[skip..len];
                self.rx[self.rx_len..self.rx_len + part.len()].copy_from_slice(part);
                self.rx_len += part.len();
                self.rcv_nxt = self.rcv_nxt.wrapping_add(part.len() as u32);
            }
        }
    }

    /// Gestisce un ACK con eventuali blocchi SACK: conferma, duplicati, ritrasmissione veloce.
    fn on_ack_full(&mut self, s: &Segment) {
        self.mark_sacked(s.sacks());
        if s.ack == self.snd_una && s.plen == 0 && self.nmeta > 0 {
            self.dupacks += 1;
            if self.dupacks == 3 {
                // Ritrasmissione veloce: primo segmento non confermato e non SACKato. Niente timeout.
                let inflight = self.in_flight();
                self.ssthresh = (inflight / 2).max(2 * MSS);
                self.cwnd = self.ssthresh;
                if let Some(i) = (0..self.nmeta).find(|&i| !self.meta[i].sacked) {
                    self.fast_rexmit = Some(self.build_rexmit(i));
                }
            }
        } else {
            self.dupacks = 0;
            self.on_ack(s.ack);
        }
    }

    /// Segna come SACKati i segmenti interamente coperti da un blocco.
    fn mark_sacked(&mut self, blocks: &[(u32, u32)]) {
        for m in self.meta[..self.nmeta].iter_mut() {
            let l = m.seq;
            let r = l.wrapping_add(m.len as u32);
            if blocks
                .iter()
                .any(|&(bl, br)| (l.wrapping_sub(bl) as i32) >= 0 && (br.wrapping_sub(r) as i32) >= 0)
            {
                m.sacked = true;
            }
        }
    }

    /// Costruisce il segmento di ritrasmissione per il segmento `i` in attesa di ACK.
    fn build_rexmit(&mut self, i: usize) -> Segment {
        let m = self.meta[i];
        self.meta[i].rexmit = true;
        let off = m.seq.wrapping_sub(self.snd_una) as usize;
        let len = m.len as usize;
        self.seg(m.seq, FLAG_ACK).with_data(&self.tx[off..off + len])
    }

    /// Prossimo segmento della ritrasmissione da timeout, se ce ne sono ancora.
    fn next_rexmit(&mut self) -> Option<Segment> {
        if !self.rexmit_active {
            return None;
        }
        while self.rexmit_next < self.nmeta {
            let i = self.rexmit_next;
            self.rexmit_next += 1;
            if !self.meta[i].sacked {
                return Some(self.build_rexmit(i));
            }
        }
        self.rexmit_active = false;
        None
    }

    /// Conferma i byte fino ad `ack`: misura il RTT, fa crescere la finestra di congestione.
    fn on_ack(&mut self, ack: u32) {
        let newly = ack.wrapping_sub(self.snd_una);
        let in_flight = self.snd_nxt.wrapping_sub(self.snd_una);
        if newly == 0 || newly > in_flight {
            return;
        }
        let n = newly as usize;
        self.snd_una = ack;
        let shift = n.min(self.tx_len);
        self.tx.copy_within(shift..self.tx_len, 0);
        self.tx_len -= shift;

        let mut sample = None;
        let mut k = 0;
        while k < self.nmeta {
            let m = self.meta[k];
            let end = m.seq.wrapping_add(m.len as u32);
            if (ack.wrapping_sub(end) as i32) < 0 {
                break;
            }
            if !m.rexmit {
                sample = Some(self.now.wrapping_sub(m.sent_at).max(1));
            }
            k += 1;
        }
        self.meta.copy_within(k..self.nmeta, 0);
        self.nmeta -= k;
        self.rexmit_next = self.rexmit_next.saturating_sub(k);

        // Congestione: slow start fino a ssthresh, poi crescita lineare (~1 MSS per RTT).
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

    /// Invia fino a `MSS` byte in stato Established, nei limiti di finestra e di congestione.
    /// Ritorna `None` se non c'è stato, se i dati sono troppi o se non ci sono slot.
    pub fn send(&mut self, data: &[u8]) -> Option<Segment> {
        if self.state != State::Established || data.len() > MSS {
            return None;
        }
        let limit = (self.peer_wnd as usize).min(self.cwnd).min(RX);
        if self.in_flight() + data.len() > limit {
            return None;
        }
        let n = data.len();
        if n > 0 {
            if self.nmeta == OUT_SLOTS {
                return None;
            }
            self.tx[self.tx_len..self.tx_len + n].copy_from_slice(data);
            self.tx_len += n;
            if self.nmeta == 0 {
                self.idle = 0;
            }
            self.meta[self.nmeta] = Meta { seq: self.snd_nxt, len: n as u8, sent_at: self.now, rexmit: false, sacked: false };
            self.nmeta += 1;
        }
        let s = self.seg(self.snd_nxt, FLAG_ACK).with_data(data);
        self.snd_nxt = self.snd_nxt.wrapping_add(n as u32);
        Some(s)
    }

    /// Un tick del timer. Ritorna al più un segmento da rimandare: SYN/SYN-ACK in sospeso,
    /// oppure, allo scadere del RTO, i segmenti non confermati e non SACKati, uno per tick.
    pub fn tick(&mut self) -> Option<Segment> {
        self.now = self.now.wrapping_add(1);
        if let Some(h) = self.hs {
            self.hs_idle += 1;
            if self.hs_idle < RTO_TICKS {
                return None;
            }
            self.hs_idle = 0;
            self.hs_tries += 1;
            if self.hs_tries > HS_MAX_TRIES {
                self.hs = None;
                self.state = State::Closed;
                return None;
            }
            return Some(h);
        }
        if let Some(s) = self.next_rexmit() {
            return Some(s);
        }
        if self.nmeta == 0 {
            self.idle = 0;
            return None;
        }
        self.idle += 1;
        if self.idle < self.rto {
            return None;
        }
        // Timeout: ssthresh a metà del volume in volo, finestra di congestione a 1 MSS, RTO raddoppiato.
        self.ssthresh = (self.in_flight() / 2).max(2 * MSS);
        self.cwnd = MSS;
        self.rto = (self.rto * 2).min(RTO_MAX);
        self.idle = 0;
        self.rexmit_active = true;
        self.rexmit_next = 0;
        self.next_rexmit()
    }

    /// La ritrasmissione veloce pronta, se c'è. Va inviata subito.
    pub fn take_fast_rexmit(&mut self) -> Option<Segment> {
        self.fast_rexmit.take()
    }

    /// Dati ricevuti in ordine e non ancora letti.
    pub fn received(&self) -> &[u8] {
        &self.rx[..self.rx_len]
    }

    /// L'applicazione legge fino a `out.len()` byte. Se il buffer si libera, ritorna un ACK per il peer.
    pub fn read(&mut self, out: &mut [u8]) -> (usize, Option<Segment>) {
        let n = out.len().min(self.rx_len);
        out[..n].copy_from_slice(&self.rx[..n]);
        self.rx.copy_within(n..self.rx_len, 0);
        self.rx_len -= n;
        let ack = if n == 0 || self.state == State::Closed {
            None
        } else {
            Some(self.seg(self.snd_nxt, FLAG_ACK))
        };
        (n, ack)
    }

    /// Chiude l'invio con FIN.
    pub fn close(&mut self) -> Option<Segment> {
        if self.state != State::Established {
            return None;
        }
        let s = self.seg(self.snd_nxt, FLAG_FIN | FLAG_ACK);
        self.snd_nxt = self.snd_nxt.wrapping_add(1);
        self.state = State::FinWait;
        Some(s)
    }

    /// Finestra di congestione corrente, in byte.
    pub fn cwnd(&self) -> usize {
        self.cwnd
    }

    /// Timeout di ritrasmissione corrente, in tick.
    pub fn rto(&self) -> u32 {
        self.rto
    }

    /// Dati inviati e non ancora confermati, in byte.
    pub fn in_flight(&self) -> usize {
        self.snd_nxt.wrapping_sub(self.snd_una) as usize
    }
}
