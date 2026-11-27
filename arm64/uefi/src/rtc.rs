//! Orologio: RTC PL031 di QEMU virt (0x09010000), secondi dal 1970 in UTC. Solo `core`.

const DR: *const u32 = 0x0901_0000 as *const u32;

/// Secondi Unix letti dall'RTC.
pub fn now() -> u32 {
    unsafe { core::ptr::read_volatile(DR) }
}

/// "HH:MM:SS" in UTC per i secondi Unix dati.
pub fn hms(epoch: u32) -> [u8; 8] {
    let s = epoch % 86_400;
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    let d = |n: u32| [b'0' + (n / 10) as u8, b'0' + (n % 10) as u8];
    let (hh, mm, ss) = (d(h), d(m), d(sec));
    [hh[0], hh[1], b':', mm[0], mm[1], b':', ss[0], ss[1]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formato_ora() {
        assert_eq!(&hms(0), b"00:00:00");
        assert_eq!(&hms(3661), b"01:01:01");
        assert_eq!(&hms(86_400 + 45_296), b"12:34:56");
    }
}
