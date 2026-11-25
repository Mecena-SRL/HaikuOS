//! Microkernel rete nel simulatore: frame ethernet e checksum IPv4 (RFC 1071).
//! Code a byte a dimensione fissa: il link non alloca. Lo stack TCP/IP è un microkernel separato
//! che usa questo livello. Nessun hardware reale.
//! Regole: docs/resource-microkernels.md, sezione 4.1.

use crate::resource::{Arch, Kind, Manifest, Microkernel, Status};

pub const MTU: usize = 1500;
pub const ETH_HEADER: usize = 14;
/// Frame massimo sul filo: intestazione più MTU.
pub const FRAME_MAX: usize = ETH_HEADER + MTU;
/// Byte per coda (ricezione e trasmissione). Ogni frame occupa 2 byte di prefisso più il frame.
pub const LINK_BYTES: usize = 4096;

pub const NET_MANIFEST: Manifest = Manifest {
    name: "net-ethernet-generic",
    kind: Kind::Network,
    archs: &[Arch::Arm64, Arch::X86_64],
    required: true,
    bytes: 8 * 1024,
    queue_depth: 16,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetError {
    TooShort,
    TooLarge,
    /// Coda piena (numero di frame o byte).
    Full,
    /// Buffer di uscita troppo piccolo.
    NoSpace,
    /// Link giù.
    Down,
}

pub use crate::csum::internet_checksum;

pub const BROADCAST: [u8; 6] = [0xff; 6];

/// Vista di un frame grezzo: il payload è una fetta di `raw`, nessuna copia.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameRef<'a> {
    pub dst: [u8; 6],
    pub src: [u8; 6],
    pub ethertype: u16,
    pub payload: &'a [u8],
}

impl<'a> FrameRef<'a> {
    pub fn parse(raw: &'a [u8]) -> Result<FrameRef<'a>, NetError> {
        if raw.len() < ETH_HEADER {
            return Err(NetError::TooShort);
        }
        if raw.len() - ETH_HEADER > MTU {
            return Err(NetError::TooLarge);
        }
        Ok(FrameRef {
            dst: raw[0..6].try_into().unwrap(),
            src: raw[6..12].try_into().unwrap(),
            ethertype: u16::from_be_bytes([raw[12], raw[13]]),
            payload: &raw[ETH_HEADER..],
        })
    }
}

/// Scrive un frame in `out`. Ritorna la lunghezza scritta.
pub fn encode_frame(
    out: &mut [u8],
    dst: &[u8; 6],
    src: &[u8; 6],
    ethertype: u16,
    payload: &[u8],
) -> Result<usize, NetError> {
    if payload.len() > MTU {
        return Err(NetError::TooLarge);
    }
    let total = ETH_HEADER + payload.len();
    if out.len() < total {
        return Err(NetError::NoSpace);
    }
    out[0..6].copy_from_slice(dst);
    out[6..12].copy_from_slice(src);
    out[12..14].copy_from_slice(&ethertype.to_be_bytes());
    out[ETH_HEADER..total].copy_from_slice(payload);
    Ok(total)
}

/// Coda FIFO di frame grezzi in un buffer circolare a byte fisso.
struct Ring {
    buf: [u8; LINK_BYTES],
    /// Indice del byte più vecchio.
    head: usize,
    /// Byte occupati, prefissi compresi.
    used: usize,
    frames: usize,
    /// Numero massimo di frame in coda.
    cap: usize,
}

impl Ring {
    const fn new(cap: usize) -> Self {
        Self { buf: [0; LINK_BYTES], head: 0, used: 0, frames: 0, cap }
    }

    fn push(&mut self, raw: &[u8]) -> Result<(), NetError> {
        let need = raw.len() + 2;
        if self.frames >= self.cap || LINK_BYTES - self.used < need {
            return Err(NetError::Full);
        }
        let mut at = (self.head + self.used) % LINK_BYTES;
        for &b in (raw.len() as u16).to_be_bytes().iter().chain(raw) {
            self.buf[at] = b;
            at = (at + 1) % LINK_BYTES;
        }
        self.used += need;
        self.frames += 1;
        Ok(())
    }

    fn pop(&mut self, out: &mut [u8; FRAME_MAX]) -> Option<usize> {
        if self.frames == 0 {
            return None;
        }
        let len = u16::from_be_bytes([self.buf[self.head], self.buf[(self.head + 1) % LINK_BYTES]]) as usize;
        let mut at = (self.head + 2) % LINK_BYTES;
        for o in out[..len].iter_mut() {
            *o = self.buf[at];
            at = (at + 1) % LINK_BYTES;
        }
        self.head = at;
        self.used -= len + 2;
        self.frames -= 1;
        Some(len)
    }
}

/// Link ethernet con stato: code di ricezione e trasmissione a byte fissi, filtro per MAC.
pub struct NetLink {
    mac: [u8; 6],
    up: bool,
    rx: Ring,
    tx: Ring,
}

impl NetLink {
    /// `depth` è il numero massimo di frame per coda; lo spazio in byte è `LINK_BYTES`.
    pub fn new(mac: [u8; 6], depth: usize) -> Self {
        Self { mac, up: true, rx: Ring::new(depth), tx: Ring::new(depth) }
    }

    pub fn set_link(&mut self, up: bool) {
        self.up = up;
    }

    /// Accoda un frame in trasmissione. Rifiuta payload oltre MTU prima di accodare.
    pub fn send(&mut self, dst: &[u8; 6], src: &[u8; 6], ethertype: u16, payload: &[u8]) -> Result<(), NetError> {
        if !self.up {
            return Err(NetError::Down);
        }
        let mut buf = [0u8; FRAME_MAX];
        let n = encode_frame(&mut buf, dst, src, ethertype, payload)?;
        self.tx.push(&buf[..n])
    }

    /// Riceve un frame grezzo. Accetta solo il proprio MAC o il broadcast. Ritorna true se accettato.
    pub fn receive(&mut self, raw: &[u8]) -> Result<bool, NetError> {
        if !self.up {
            return Err(NetError::Down);
        }
        let f = FrameRef::parse(raw)?;
        if f.dst != self.mac && f.dst != BROADCAST {
            return Ok(false);
        }
        self.rx.push(raw)?;
        Ok(true)
    }

    /// Estrae il prossimo frame ricevuto in `out`; ritorna la sua lunghezza.
    pub fn next_rx(&mut self, out: &mut [u8; FRAME_MAX]) -> Option<usize> {
        self.rx.pop(out)
    }

    /// Estrae il prossimo frame da trasmettere in `out`; ritorna la sua lunghezza.
    pub fn next_tx(&mut self, out: &mut [u8; FRAME_MAX]) -> Option<usize> {
        self.tx.pop(out)
    }
}

impl Microkernel for NetLink {
    fn manifest(&self) -> Manifest {
        NET_MANIFEST
    }
    fn status(&self) -> Status {
        if self.up { Status::Ready } else { Status::Stopped }
    }
}
