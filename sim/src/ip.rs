//! IPv4 senza allocazione (no_std: solo `core`). Intestazione senza opzioni, checksum verificato.
//! Sta sopra il link (ethertype 0x0800). Nessun hardware reale.

use crate::csum::internet_checksum;

pub const ETHERTYPE_IPV4: u16 = 0x0800;
pub const PROTO_TCP: u8 = 6;
pub const HEADER: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpError {
    TooShort,
    BadVersion,
    BadChecksum,
    BadLength,
    /// Il buffer di uscita è troppo piccolo.
    NoSpace,
}

/// Pacchetto IPv4 che prende in prestito il payload: nessuna copia.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Packet<'a> {
    pub src: [u8; 4],
    pub dst: [u8; 4],
    pub proto: u8,
    pub payload: &'a [u8],
}

impl<'a> Packet<'a> {
    /// Scrive intestazione e payload in `out`. Ritorna la lunghezza scritta.
    pub fn encode_into(&self, out: &mut [u8]) -> Result<usize, IpError> {
        let total = HEADER + self.payload.len();
        if total > u16::MAX as usize {
            return Err(IpError::BadLength);
        }
        if out.len() < total {
            return Err(IpError::NoSpace);
        }
        let h = &mut out[..HEADER];
        h.fill(0);
        h[0] = 0x45; // versione 4, IHL 5
        h[2..4].copy_from_slice(&(total as u16).to_be_bytes());
        h[8] = 64; // TTL
        h[9] = self.proto;
        h[12..16].copy_from_slice(&self.src);
        h[16..20].copy_from_slice(&self.dst);
        let c = internet_checksum(h);
        h[10..12].copy_from_slice(&c.to_be_bytes());
        out[HEADER..total].copy_from_slice(self.payload);
        Ok(total)
    }

    /// Legge un pacchetto; il payload è una fetta di `raw`.
    pub fn parse(raw: &'a [u8]) -> Result<Packet<'a>, IpError> {
        if raw.len() < HEADER {
            return Err(IpError::TooShort);
        }
        if raw[0] >> 4 != 4 || raw[0] & 0x0f != 5 {
            return Err(IpError::BadVersion);
        }
        let total = u16::from_be_bytes([raw[2], raw[3]]) as usize;
        if total < HEADER || total > raw.len() {
            return Err(IpError::BadLength);
        }
        // Il checksum di un'intestazione valida, sommato al proprio campo, dà 0.
        if internet_checksum(&raw[..HEADER]) != 0 {
            return Err(IpError::BadChecksum);
        }
        Ok(Packet {
            src: raw[12..16].try_into().unwrap(),
            dst: raw[16..20].try_into().unwrap(),
            proto: raw[9],
            payload: &raw[HEADER..total],
        })
    }
}
