//! Livello sottile GBA: la SRAM della cartuccia (32 KB, byte per byte) come supporto di persistenza.
use crate::persist::{Medium, Profile};
use core::ptr::{read_volatile, write_volatile};

pub const SIZE: usize = 32 * 1024;
const BASE: usize = 0x0E00_0000;

pub struct Sram;

impl Medium for Sram {
    fn read(&self, off: usize, out: &mut [u8]) {
        for (i, b) in out.iter_mut().enumerate() {
            *b = unsafe { read_volatile((BASE + off + i) as *const u8) };
        }
    }
    fn write(&mut self, off: usize, data: &[u8]) {
        for (i, &b) in data.iter().enumerate() {
            unsafe { write_volatile((BASE + off + i) as *mut u8, b) };
        }
    }
    /// Salvataggio a 64 modifiche o ogni 30 s, al massimo 2 al minuto. Usura della SRAM: da verificare.
    fn profile(&self) -> Profile {
        Profile { units: 64, max_frames: 1800, max_writes_per_min: 2 }
    }
}

/// Aspetta il vblank: un frame ogni 1/60 s. Base per il conteggio dei frame.
pub fn vblank() {
    const VCOUNT: *const u16 = 0x0400_0006 as *const u16;
    unsafe {
        while read_volatile(VCOUNT) >= 160 {}
        while read_volatile(VCOUNT) < 160 {}
    }
}
