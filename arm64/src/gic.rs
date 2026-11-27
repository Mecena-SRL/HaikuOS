//! GICv2 (QEMU virt): distributore 0x08000000, interfaccia CPU 0x08010000.
use core::ptr::{read_volatile as rd, write_volatile as wr};

const GICD: usize = 0x0800_0000;
const GICC: usize = 0x0801_0000;

/// Abilita il distributore e l'interfaccia CPU, poi l'interrupt `id` (SPI, CPU 0, priorità media).
pub fn enable_irq(id: u32) {
    unsafe {
        wr((GICD) as *mut u32, 1); // CTLR: distributore on
        wr((GICD + 0x100 + 4 * (id as usize / 32)) as *mut u32, 1 << (id % 32)); // ISENABLER
        wr((GICD + 0x400 + id as usize) as *mut u8, 0x80); // IPRIORITYR
        wr((GICD + 0x800 + id as usize) as *mut u8, 1); // ITARGETSR: CPU 0
        wr((GICC) as *mut u32, 1); // CTLR CPU: on
        wr((GICC + 0x04) as *mut u32, 0xFF); // PMR: accetta tutte le priorità
    }
}

/// Legge l'id dell'interrupt attivo (IAR). Va chiuso con `eoi`.
pub fn ack() -> u32 {
    unsafe { rd((GICC + 0x0C) as *const u32) }
}

pub fn eoi(iar: u32) {
    unsafe { wr((GICC + 0x10) as *mut u32, iar) }
}
