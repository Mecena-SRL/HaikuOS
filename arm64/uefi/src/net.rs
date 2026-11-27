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
    Some(Nic { mac, up: !supported || present })
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
