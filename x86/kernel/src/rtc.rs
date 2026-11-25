//! Orologio di sistema: RTC CMOS (porte 0x70/0x71), ore e minuti. Usa BCD o binario secondo il registro B.
use crate::io::{inb, outb};

fn read(reg: u8) -> u8 {
    unsafe {
        outb(0x70, reg);
        inb(0x71)
    }
}

/// Ore e minuti attuali (24 ore: da verificare con la modalità 12 ore del CMOS).
pub fn time() -> (u8, u8) {
    let (h, m, b) = (read(0x04), read(0x02), read(0x0B));
    if b & 0x04 != 0 {
        (h, m)
    } else {
        (bcd(h), bcd(m))
    }
}

fn bcd(v: u8) -> u8 {
    (v >> 4) * 10 + (v & 0x0F)
}
