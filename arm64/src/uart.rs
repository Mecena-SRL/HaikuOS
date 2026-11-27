//! PL011 (0x09000000): ricezione con interrupt (IRQ 33 su QEMU virt).
use core::ptr::{read_volatile as rd, write_volatile as wr};

pub const BASE: usize = 0x0900_0000;
pub const IRQ: u32 = 33;

pub fn rx_irq_on() {
    unsafe {
        wr((BASE + 0x30) as *mut u32, 0x301); // CR: UARTEN, TXE, RXE
        wr((BASE + 0x38) as *mut u32, 1 << 4); // IMSC: RXIM
    }
}

pub fn clear_irq() {
    unsafe { wr((BASE + 0x44) as *mut u32, 0x7FF) } // ICR
}

/// Svuota la coda di ricezione; ritorna i byte letti.
pub fn read_all(mut f: impl FnMut(u8)) {
    unsafe {
        while rd((BASE + 0x18) as *const u32) & (1 << 4) == 0 {
            f(rd(BASE as *const u32) as u8);
        }
    }
}
