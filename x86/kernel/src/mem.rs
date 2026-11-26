//! Mappa memoria BIOS (E820) lasciata da stage2 a 0x0A00: conteggio, poi voci da 24 byte.
const COUNT: usize = 0x0A00;
const ENTRIES: usize = 0x0A10;

/// Byte di RAM utilizzabile (tipo 1, voci della mappa BIOS).
pub fn usable() -> u64 {
    let mut usable = 0u64;
    unsafe {
        let n = core::ptr::read_volatile(COUNT as *const u32) as usize;
        for i in 0..n.min(16) {
            let e = ENTRIES + i * 24;
            let len = core::ptr::read_volatile((e + 8) as *const u64);
            let kind = core::ptr::read_volatile((e + 16) as *const u32);
            if kind == 1 {
                usable += len;
            }
        }
    }
    usable
}
