//! Tempo senza interrupt: TSC (rdtsc) calibrato sul PIT (canale 0, 100 Hz) in polling.
//! Nessuna allocazione; µs interi da `now_us()`.
use crate::io::{inb, outb};
use core::arch::asm;

/// Periodo di calibrazione: 11932 cicli del PIT ≈ 10 ms.
const CAL_COUNT: u16 = 11_932;
const PIT_HZ: u64 = 1_193_182;

static mut TSC_HZ: u64 = 1_000_000_000;

#[inline]
fn rdtsc() -> u64 {
    let (lo, hi): (u32, u32);
    unsafe { asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack)) };
    ((hi as u64) << 32) | lo as u64
}

/// Valore corrente del contatore PIT canale 0 (latch, poi due letture).
fn pit_now() -> u16 {
    unsafe {
        outb(0x43, 0x00);
        let lo = inb(0x40) as u16;
        let hi = inb(0x40) as u16;
        (hi << 8) | lo
    }
}

/// Attende il ricaricamento del PIT: il contatore scende e un salto in su segna un periodo.
fn wait_reload() {
    let mut prev = pit_now();
    loop {
        let v = pit_now();
        if v > prev {
            return;
        }
        prev = v;
    }
}

/// Calibra il TSC: programma il PIT a 10 ms e misura i TSC per un periodo intero. Ritorna MHz.
pub fn init() -> u32 {
    unsafe {
        outb(0x43, 0x34); // canale 0, lo/hi, modo 2 (rigenerazione), binario
        outb(0x40, (CAL_COUNT & 0xFF) as u8);
        outb(0x40, (CAL_COUNT >> 8) as u8);
    }
    wait_reload();
    let t0 = rdtsc();
    wait_reload();
    let t1 = rdtsc();
    let hz = (t1 - t0) * PIT_HZ / CAL_COUNT as u64;
    unsafe { TSC_HZ = hz.max(1) };
    (hz / 1_000_000) as u32
}

/// Microsecondi dall'avvio del TSC (u64: non si azzera per ore).
pub fn now_us() -> u64 {
    let hz = unsafe { TSC_HZ } as u128;
    (rdtsc() as u128 * 1_000_000 / hz) as u64
}
