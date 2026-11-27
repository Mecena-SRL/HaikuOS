//! Rete: scheda di rete tramite EFI_SIMPLE_NETWORK_PROTOCOL del firmware (driver SNP già nel firmware).
//! GUID SNP: {a19832b9-ac25-11d3-9a2d-0090273fc14d}.
//! Trova la scheda, la avvia, legge MAC e stato del collegamento. Niente stack TCP/IP ancora.

use core::ffi::c_void;


// EFI_SIMPLE_NETWORK_PROTOCOL: Start a 8, Initialize a 24, Mode (puntatore) a 120.
// EFI_SIMPLE_NETWORK_MODE: State u32 a 0, MCastFilter[16] da 40 (EFI_MAC_ADDRESS = 32 byte),
// CurrentAddress a 552, MediaPresentSupported u8 a 651, MediaPresent u8 a 652.
type Start = extern "efiapi" fn(*mut c_void) -> usize;
type Initialize = extern "efiapi" fn(*mut c_void, usize, usize) -> usize;

const STATE_STOPPED: u32 = 0;
const STATE_INITIALIZED: u32 = 2;

#[derive(Clone, Copy)]
pub struct Nic {
    pub mac: [u8; 6],
    pub up: bool,
    pub snp: *mut c_void,
}

/// Cerca la scheda SNP, la avvia se serve e legge MAC e collegamento.
/// `locate` chiama LocateProtocol con il GUID SNP e scrive l'interfaccia in `out`.
pub unsafe fn detect(locate: impl FnOnce(*mut *mut c_void) -> usize) -> Option<Nic> {
    let mut snp: *mut c_void = core::ptr::null_mut();
    if locate(&mut snp) != 0 || snp.is_null() {
        return None;
    }
    let base = snp as *mut u8;
    let mode = *(base.add(120) as *const *mut u8);
    if mode.is_null() {
        return None;
    }
    if *(mode as *const u32) == STATE_STOPPED {
        let start = *(base.add(8) as *const Start);
        start(snp);
    }
    if *(mode as *const u32) != STATE_INITIALIZED {
        let init = *(base.add(24) as *const Initialize);
        init(snp, 0, 0);
    }
    let mut mac = [0u8; 6];
    core::ptr::copy_nonoverlapping(mode.add(552), mac.as_mut_ptr(), 6);
    let supported = *mode.add(651) != 0;
    let present = *mode.add(652) != 0;
    Some(Nic { mac, up: !supported || present, snp })
}

type Transmit = extern "efiapi" fn(*mut c_void, usize, usize, *const u8, *const u8, *const u8, *const u16) -> usize;
type GetStatus = extern "efiapi" fn(*mut c_void, *mut u32, *mut *mut c_void) -> usize;
type Receive = extern "efiapi" fn(*mut c_void, *mut usize, *mut usize, *mut u8, *mut u8, *mut u8, *mut u16) -> usize;

/// Invia un frame Ethernet completo (intestazione inclusa). SNP Transmit a 96.
pub fn send(nic: &Nic, frame: &[u8]) {
    unsafe {
        let tx = *((nic.snp as *const u8).add(96) as *const Transmit);
        tx(nic.snp, 0, frame.len(), frame.as_ptr(), core::ptr::null(), core::ptr::null(), core::ptr::null());
        // Riciclo dei buffer di trasmissione completati (GetStatus a 88), altrimenti la coda si blocca.
        let gs = *((nic.snp as *const u8).add(88) as *const GetStatus);
        loop {
            let mut irq: u32 = 0;
            let mut done: *mut c_void = core::ptr::null_mut();
            gs(nic.snp, &mut irq, &mut done);
            if done.is_null() {
                break;
            }
        }
    }
}

/// Riceve un frame se c'è, senza attendere. SNP Receive a 104.
pub fn recv(nic: &Nic, buf: &mut [u8]) -> Option<usize> {
    unsafe {
        let rx = *((nic.snp as *const u8).add(104) as *const Receive);
        let mut hdr: usize = 0;
        let mut len = buf.len();
        let ptr = buf.as_mut_ptr();
        if rx(nic.snp, &mut hdr, &mut len, ptr, core::ptr::null_mut(), core::ptr::null_mut(), core::ptr::null_mut()) == 0 {
            Some(len)
        } else {
            None
        }
    }
}

/// Millisecondi dal contatore generico ARM (CNTPCT_EL0 / CNTFRQ_EL0).
pub fn now_ms() -> u64 {
    let (c, f): (u64, u64);
    unsafe {
        core::arch::asm!("mrs {}, cntpct_el0", out(reg) c);
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) f);
    }
    c * 1000 / f
}

/// MAC come "52:54:00:12:34:56" in `out`.
pub fn mac_str<'a>(mac: &[u8; 6], out: &'a mut [u8; 17]) -> &'a str {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for i in 0..6 {
        out[i * 3] = HEX[(mac[i] >> 4) as usize];
        out[i * 3 + 1] = HEX[(mac[i] & 0xF) as usize];
        if i < 5 {
            out[i * 3 + 2] = b':';
        }
    }
    core::str::from_utf8(out).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_formato() {
        let mut out = [0u8; 17];
        assert_eq!(mac_str(&[0x52, 0x54, 0x00, 0x12, 0x34, 0xAB], &mut out), "52:54:00:12:34:AB");
    }
}
