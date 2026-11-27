//! Tastiera UEFI (EFI_SIMPLE_TEXT_INPUT_PROTOCOL) in modalità polling.

use core::ffi::c_void;

// EFI_SYSTEM_TABLE: ConIn a 48. SIMPLE_TEXT_INPUT_PROTOCOL: ReadKeyStroke a 8.
type ReadKeyStroke = extern "efiapi" fn(*mut c_void, *mut Key) -> usize;

#[repr(C)]
#[derive(Default)]
pub struct Key {
    pub scan: u16,
    pub unicode: u16,
}


/// Legge un tasto se disponibile. None = nessun tasto (EFI_NOT_READY) o errore.
pub fn read_key(st: *mut c_void) -> Option<Key> {
    unsafe {
        let con_in = *((st as *const u8).add(48) as *const *mut c_void);
        let read = *((con_in as *const usize).add(1) as *const ReadKeyStroke);
        let mut k = Key::default();
        if read(con_in, &mut k) == 0 {
            Some(k)
        } else {
            None
        }
    }
}
