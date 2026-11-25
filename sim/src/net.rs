//! Microkernel rete nel simulatore: frame ethernet e checksum IPv4 (RFC 1071).
//! Lo stack TCP/IP è un microkernel separato che usa questo livello. Nessun hardware reale.
//! Regole: docs/resource-microkernels.md, sezione 4.1.

use crate::resource::{Arch, Error as QError, Kind, Manifest, Microkernel, Queue, Status};

pub const MTU: usize = 1500;
pub const ETH_HEADER: usize = 14;

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
    /// Coda piena.
    Full,
    /// Link giù.
    Down,
}

impl From<QError> for NetError {
    fn from(_: QError) -> Self {
        NetError::Full
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub dst: [u8; 6],
    pub src: [u8; 6],
    pub ethertype: u16,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn encode(&self) -> Result<Vec<u8>, NetError> {
        if self.payload.len() > MTU {
            return Err(NetError::TooLarge);
        }
        let mut out = Vec::with_capacity(ETH_HEADER + self.payload.len());
        out.extend_from_slice(&self.dst);
        out.extend_from_slice(&self.src);
        out.extend_from_slice(&self.ethertype.to_be_bytes());
        out.extend_from_slice(&self.payload);
        Ok(out)
    }

    pub fn parse(raw: &[u8]) -> Result<Frame, NetError> {
        if raw.len() < ETH_HEADER {
            return Err(NetError::TooShort);
        }
        if raw.len() - ETH_HEADER > MTU {
            return Err(NetError::TooLarge);
        }
        Ok(Frame {
            dst: raw[0..6].try_into().unwrap(),
            src: raw[6..12].try_into().unwrap(),
            ethertype: u16::from_be_bytes([raw[12], raw[13]]),
            payload: raw[ETH_HEADER..].to_vec(),
        })
    }
}

pub use crate::csum::internet_checksum;

pub const BROADCAST: [u8; 6] = [0xff; 6];

/// Link ethernet con stato: code di ricezione e trasmissione a profondità fissa, filtro per MAC.
pub struct NetLink {
    mac: [u8; 6],
    up: bool,
    rx: Queue<Frame>,
    tx: Queue<Frame>,
}

impl NetLink {
    pub fn new(mac: [u8; 6], depth: usize) -> Self {
        Self { mac, up: true, rx: Queue::new(depth), tx: Queue::new(depth) }
    }

    pub fn set_link(&mut self, up: bool) {
        self.up = up;
    }

    /// Accoda un frame in trasmissione.
    pub fn send(&mut self, f: Frame) -> Result<(), NetError> {
        if !self.up {
            return Err(NetError::Down);
        }
        f.encode()?; // rifiuta frame oltre MTU prima di accodare
        Ok(self.tx.push(f)?)
    }

    /// Riceve un frame grezzo. Accetta solo il proprio MAC o il broadcast. Ritorna true se accettato.
    pub fn receive(&mut self, raw: &[u8]) -> Result<bool, NetError> {
        if !self.up {
            return Err(NetError::Down);
        }
        let f = Frame::parse(raw)?;
        if f.dst != self.mac && f.dst != BROADCAST {
            return Ok(false);
        }
        self.rx.push(f)?;
        Ok(true)
    }

    pub fn next_rx(&mut self) -> Option<Frame> {
        self.rx.pop()
    }

    pub fn next_tx(&mut self) -> Option<Frame> {
        self.tx.pop()
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
