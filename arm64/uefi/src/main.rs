//! Applicazione UEFI di HaikuOS (ARM64): trova il GOP, disegna il desktop e stampa su ConOut.
#![no_std]
#![no_main]

mod gfx;

use core::ffi::c_void;
use core::panic::PanicInfo;
use gfx::{Fb, Layout};

// EFI_SYSTEM_TABLE: BootServices a 96, ConOut a 64. SIMPLE_TEXT_OUTPUT_PROTOCOL: OutputString a 8.
// EFI_BOOT_SERVICES: LocateProtocol a 0x140, Stall a 0xF8.
type OutputString = extern "efiapi" fn(*mut c_void, *const u16) -> usize;
type LocateProtocol = extern "efiapi" fn(*const Guid, *mut c_void, *mut *mut c_void) -> usize;
type Stall = extern "efiapi" fn(usize) -> usize;

#[repr(C)]
struct Guid {
    d1: u32,
    d2: u16,
    d3: u16,
    d4: [u8; 8],
}

// EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID {9042a9de-23dc-4a38-96fb-7aded080516a}
const GOP_GUID: Guid = Guid {
    d1: 0x9042_a9de,
    d2: 0x23dc,
    d3: 0x4a38,
    d4: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

fn con_out_print(st: *mut c_void, msg: &[u16]) {
    unsafe {
        let con_out = *((st as *const u8).add(64) as *const *mut c_void);
        let out_fn = *((con_out as *const usize).add(1) as *const OutputString);
        out_fn(con_out, msg.as_ptr());
    }
}

/// ASCII in UTF-16 nel buffer, terminato da zero. Il buffer è di 96 unità.
fn wide(s: &str, buf: &mut [u16; 96]) {
    for (i, b) in s.bytes().enumerate() {
        buf[i] = b as u16;
    }
    buf[s.len()] = 0;
}

/// Scrive `n` in decimale da `at`, ritorna la posizione successiva (senza zero finale).
fn push_dec(buf: &mut [u16; 96], mut at: usize, mut n: u32) -> usize {
    let mut tmp = [0u8; 10];
    let mut i = tmp.len();
    loop {
        i -= 1;
        tmp[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    for &b in &tmp[i..] {
        buf[at] = b as u16;
        at += 1;
    }
    at
}

#[no_mangle]
pub extern "efiapi" fn efi_main(_image: *mut c_void, st: *mut c_void) -> usize {
    let mut buf = [0u16; 96];
    unsafe {
        let bs = *((st as *const u8).add(96) as *const *const u8);
        let locate = *(bs.add(0x140) as *const LocateProtocol);
        let mut gop: *mut c_void = core::ptr::null_mut();
        if locate(&GOP_GUID, core::ptr::null_mut(), &mut gop) != 0 || gop.is_null() {
            wide("[errore] GOP non trovato\r\n", &mut buf);
            con_out_print(st, &buf);
            return 0;
        }
        // EFI_GRAPHICS_OUTPUT_PROTOCOL: Mode a 24. Mode: Info a 8, FrameBufferBase a 24.
        // Info: H a 4, V a 8, PixelFormat a 12, PixelInformation (maschere R,G,B) a 16..28, PixelsPerScanLine a 32.
        let mode = *((gop as *const u8).add(24) as *const *const u8);
        let info = *(mode.add(8) as *const *const u8);
        let (w, h) = (*(info.add(4) as *const u32), *(info.add(8) as *const u32));
        let fmt = *(info.add(12) as *const u32);
        let ppl = *(info.add(32) as *const u32);
        let layout = match fmt {
            0 => Layout::RGB8,
            1 => Layout::BGR8,
            2 => {
                let m = |o: usize| *(info.add(o) as *const u32);
                Layout::from_masks(m(16), m(20), m(24))
            }
            _ => {
                wide("[errore] GOP senza accesso diretto al framebuffer\r\n", &mut buf);
                con_out_print(st, &buf);
                return 0;
            }
        };
        let fb = Fb {
            base: *(mode.add(24) as *const u64) as *mut u8,
            w,
            h,
            ppl,
            layout,
        };
        // Messaggi prima del disegno: il firmware scrive il testo sullo stesso schermo.
        wide("HaikuOS 0.1 - UEFI ARM64\r\n", &mut buf);
        con_out_print(st, &buf);
        wide("[ok] desktop GOP ", &mut buf);
        let mut at = push_dec(&mut buf, 17, w);
        buf[at] = b'x' as u16;
        at = push_dec(&mut buf, at + 1, h);
        buf[at] = b' ' as u16;
        at = push_dec(&mut buf, at + 1, layout.bytes * 8);
        buf[at..at + 5].copy_from_slice(&[b' ' as u16, b'b' as u16, b'i' as u16, b't' as u16, b'\r' as u16]);
        buf[at + 5] = b'\n' as u16;
        buf[at + 6] = 0;
        con_out_print(st, &buf);
        gfx::draw_desktop(&fb);

        // Attesa di 30 s (Stall): il firmware non ridisegna lo schermo.
        let stall = *(bs.add(0xF8) as *const Stall);
        stall(30_000_000);
    }
    0 // EFI_SUCCESS
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
