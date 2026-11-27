//! Applicazione UEFI di HaikuOS (ARM64): trova il framebuffer GOP, disegna lo sfondo e stampa su ConOut.
#![no_std]
#![no_main]

use core::ffi::c_void;
use core::panic::PanicInfo;

// EFI_SYSTEM_TABLE: BootServices a 96, ConOut a 64. SIMPLE_TEXT_OUTPUT_PROTOCOL: OutputString a 8.
// EFI_BOOT_SERVICES: LocateProtocol a 0x140.
type OutputString = extern "efiapi" fn(*mut c_void, *const u16) -> usize;
type LocateProtocol = extern "efiapi" fn(*const Guid, *mut c_void, *mut *mut c_void) -> usize;

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

const BG: u32 = 0x08_10_31; // tema 0 di HaikuOS (RRGGBB)
const ACCENT: u32 = 0xFF_84_00;

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

/// Scrive `n` in decimale in coda a `buf[at..]`, ritorna la nuova posizione (senza zero finale).
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

/// Disegna sfondo e un rettangolo accento. `rgb_order` = true per formato RGB (byte R,G,B).
unsafe fn draw(fb: *mut u32, w: u32, h: u32, ppl: u32, rgb_order: bool) {
    let px = |c: u32| if rgb_order { ((c & 0xFF) << 16) | (c & 0xFF00) | (c >> 16) } else { c };
    for y in 0..h {
        for x in 0..w {
            let c = if x >= w / 4 && x < w / 4 * 3 && y >= h / 4 && y < h / 4 * 3 { ACCENT } else { BG };
            fb.add((y * ppl + x) as usize).write_volatile(px(c));
        }
    }
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
        let mode = *((gop as *const u8).add(24) as *const *const u8);
        let info = *(mode.add(8) as *const *const u8);
        let (w, h) = (*(info.add(4) as *const u32), *(info.add(8) as *const u32));
        let fmt = *(info.add(12) as *const u32); // 0 = RGB, 1 = BGR
        let ppl = *(info.add(32) as *const u32);
        let fb = *(mode.add(24) as *const u64) as *mut u32;
        draw(fb, w, h, ppl, fmt == 0);

        wide("HaikuOS 0.1 - UEFI ARM64\r\n", &mut buf);
        con_out_print(st, &buf);
        wide("[ok] GOP ", &mut buf);
        let mut at = push_dec(&mut buf, 9, w);
        buf[at] = b'x' as u16;
        at = push_dec(&mut buf, at + 1, h);
        buf[at] = b'\r' as u16;
        buf[at + 1] = b'\n' as u16;
        buf[at + 2] = 0;
        con_out_print(st, &buf);
        // Attesa di 30 s (BootServices.Stall, offset 0xF8): il firmware non ridisegna lo schermo.
        let stall = *(bs.add(0xF8) as *const extern "efiapi" fn(usize) -> usize);
        stall(30_000_000);
    }
    0 // EFI_SUCCESS
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
