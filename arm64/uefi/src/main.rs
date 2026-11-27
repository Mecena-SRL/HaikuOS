//! Applicazione UEFI di HaikuOS (ARM64): stampa su ConOut del firmware, poi ritorna.
#![no_std]
#![no_main]

use core::ffi::c_void;
use core::panic::PanicInfo;

// EFI_SYSTEM_TABLE: ConOut a offset 64. SIMPLE_TEXT_OUTPUT_PROTOCOL: OutputString a offset 8.
type OutputString = extern "efiapi" fn(*mut c_void, *const u16) -> usize;

fn con_out_print(st: *mut c_void, msg: &[u16]) {
    unsafe {
        let con_out = *((st as *const u8).add(64) as *const *mut c_void);
        let out_fn = *((con_out as *const usize).add(1) as *const OutputString);
        out_fn(con_out, msg.as_ptr());
    }
}

/// ASCII in UTF-16 nel buffer, terminato da zero. Il buffer è di 64 unità.
fn wide(s: &str, buf: &mut [u16; 64]) {
    for (i, b) in s.bytes().enumerate() {
        buf[i] = b as u16;
    }
    buf[s.len()] = 0;
}

#[no_mangle]
pub extern "efiapi" fn efi_main(_image: *mut c_void, st: *mut c_void) -> usize {
    let mut buf = [0u16; 64];
    wide("HaikuOS 0.1 - UEFI ARM64\r\n", &mut buf);
    con_out_print(st, &buf);
    wide("[ok] firmware UEFI: ConOut attivo\r\n", &mut buf);
    con_out_print(st, &buf);
    0 // EFI_SUCCESS
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
