//! MMU con mappatura identità: 0..1 GB device (UART), 1..2 GB RAM normale. Blocchi da 1 GB, granule 4 KB.
use core::arch::asm;
use core::ptr::addr_of_mut;

#[repr(C, align(4096))]
struct Table([u64; 512]);

static mut L1: Table = Table([0; 512]);

const BLOCK: u64 = 0b01;
const AF: u64 = 1 << 10;
const SH_INNER: u64 = 0b11 << 8;
const UXN_PXN: u64 = (1 << 54) | (1 << 53);

/// Attiva la MMU. Ritorna true se SCTLR_EL1 riporta M attivo.
pub fn enable() -> bool {
    unsafe {
        let l1 = addr_of_mut!(L1) as *mut u64;
        // Attributo 0: device nGnRnE (0x00). Attributo 1: normale write-back (0xFF).
        // Blocco 0: device, no shareability. Blocco 1 (RAM): normale, inner shareable.
        *l1.add(0) = 0x0 | BLOCK | AF | UXN_PXN;
        *l1.add(1) = 0x4000_0000 | BLOCK | AF | SH_INNER | (1 << 2); // RAM eseguibile dal kernel (PXN=0)
        let mair: u64 = 0xFF00;
        // T0SZ=25 (39 bit), TG0=4 KB, IRGN0/ORGN0=WB, SH0=inner, EPD1=1, IPS=40 bit.
        let tcr: u64 = 25 | (1 << 8) | (1 << 10) | (3 << 12) | (1 << 23) | (2 << 32);
        asm!("msr mair_el1, {}", in(reg) mair);
        asm!("msr tcr_el1, {}", in(reg) tcr);
        asm!("msr ttbr0_el1, {}", in(reg) l1 as u64);
        asm!("dsb ish", "tlbi vmalle1", "dsb ish", "ic iallu", "dsb sy", "isb");
        let mut sctlr: u64;
        asm!("mrs {}, sctlr_el1", out(reg) sctlr);
        sctlr |= (1 << 0) | (1 << 2) | (1 << 12); // M, C, I
        asm!("msr sctlr_el1, {}", "isb", in(reg) sctlr);
        let now: u64;
        asm!("mrs {}, sctlr_el1", out(reg) now);
        now & 1 == 1
    }
}
