//! Timer generico ARM (CNTPCT_EL0 / CNTFRQ_EL0), letto in polling: niente interrupt.
use core::arch::asm;

pub fn freq() -> u64 {
    let f: u64;
    unsafe { asm!("mrs {}, cntfrq_el0", out(reg) f) };
    f
}

pub fn count() -> u64 {
    let c: u64;
    unsafe { asm!("mrs {}, cntpct_el0", out(reg) c) };
    c
}

/// Attesa di `ms` millisecondi con il contatore fisico. Ritorna i tick misurati.
pub fn busy_wait_ms(ms: u64) -> u64 {
    let start = count();
    let ticks = freq() / 1000 * ms;
    while count().wrapping_sub(start) < ticks {}
    count().wrapping_sub(start)
}

/// Timer fisico EL1 con interrupt (PPI 30): scatta ogni `ms` millisecondi finché non si riarma.
pub const IRQ: u32 = 30;

pub fn arm_tick(ms: u64) {
    unsafe {
        asm!("msr cntp_tval_el0, {}", in(reg) freq() / 1000 * ms);
        asm!("msr cntp_ctl_el0, {}", in(reg) 1u64); // ENABLE=1, IMASK=0
    }
}
