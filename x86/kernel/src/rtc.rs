//! Orologio di sistema: RTC CMOS (porte 0x70/0x71) in UTC, convertito in ora locale Europa/Roma
//! (CET +1, CEST +2 con ora legale UE: ultima domenica di marzo 01:00 UTC → ultima domenica di ottobre 01:00 UTC).
use crate::io::{inb, outb};

fn read(reg: u8) -> u8 {
    unsafe {
        outb(0x70, reg);
        inb(0x71)
    }
}

/// Ore e minuti locali. Il CMOS è in 24 ore (da verificare con la modalità 12 ore).
pub fn time() -> (u8, u8) {
    let bin = read(0x0B) & 0x04 != 0;
    let dec = |v: u8| if bin { v } else { (v >> 4) * 10 + (v & 0x0F) };
    let (h, m) = (dec(read(0x04)), dec(read(0x02)));
    let (d, mo, y) = (dec(read(0x07)) as u32, dec(read(0x08)) as u32, 2000 + dec(read(0x09)) as u32);
    let off = if dst(y, mo, d, h as u32) { 2 } else { 1 };
    ((h + off) % 24, m)
}

/// Giorno della settimana, 0 = domenica (algoritmo di Sakamoto).
fn dow(y: u32, m: u32, d: u32) -> u32 {
    const T: [u32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if m < 3 { y - 1 } else { y };
    (y + y / 4 - y / 100 + y / 400 + T[(m - 1) as usize] + d) % 7
}

/// Giorno del mese dell'ultima domenica di marzo (m = 3) o ottobre (m = 10).
fn last_sunday(y: u32, m: u32) -> u32 {
    31 - dow(y, m, 31)
}

/// Ora legale in Europa, data e ora UTC.
fn dst(y: u32, mo: u32, d: u32, h: u32) -> bool {
    match mo {
        4..=9 => true,
        3 => d > last_sunday(y, 3) || (d == last_sunday(y, 3) && h >= 1),
        10 => d < last_sunday(y, 10) || (d == last_sunday(y, 10) && h < 1),
        _ => false,
    }
}
