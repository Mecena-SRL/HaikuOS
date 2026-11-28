//! Interfaccia verso il dispositivo. Due adattatori previsti:
//!
//! 1. Host (test): finto dispositivo in `tests/host.rs`.
//! 2. Kernel (STUB, non compilato qui, nessun accesso hardware reale):
//!
//! ```text
//! struct KernelBus { base: *mut u8 }
//! impl Bus for KernelBus {
//!     fn read_reg(&mut self, off: u32) -> u32 {
//!         // MMIO volatile all'indirizzo base mappato dal kernel
//!         unsafe { core::ptr::read_volatile(self.base.add(off as usize) as *const u32) }
//!     }
//!     fn write_reg(&mut self, off: u32, v: u32) {
//!         unsafe { core::ptr::write_volatile(self.base.add(off as usize) as *mut u32, v) }
//!     }
//!     fn wait(&mut self) { core::hint::spin_loop() } // o yield allo scheduler
//! }
//! ```

pub trait Bus {
    /// Legge il registro a `offset` (byte dall'inizio del dispositivo).
    fn read_reg(&mut self, offset: u32) -> u32;
    /// Scrive `value` nel registro a `offset`.
    fn write_reg(&mut self, offset: u32, value: u32);
    /// Gancio di attesa tra due poll dello stato.
    fn wait(&mut self);
}
