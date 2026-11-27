//! Applicazione UEFI di HaikuOS (ARM64): trova il GOP, disegna il desktop e stampa su ConOut.
#![no_std]
#![no_main]

mod calc;
mod gfx;
mod term;
mod input;

use core::ffi::c_void;
use core::panic::PanicInfo;
use calc::{Calc, Op};
use term::Term;
use gfx::{Fb, Layout, View};

// EFI_SYSTEM_TABLE: BootServices a 96, ConOut a 64. SIMPLE_TEXT_OUTPUT_PROTOCOL: OutputString a 8.
// EFI_BOOT_SERVICES: LocateProtocol a 0x140, Stall a 0xF8.
type OutputString = extern "efiapi" fn(*mut c_void, *const u16) -> usize;
type LocateProtocol = extern "efiapi" fn(*const Guid, *mut c_void, *mut *mut c_void) -> usize;
type Stall = extern "efiapi" fn(usize) -> usize;

/// Log su seriale PL011 (il firmware scriverebbe sopra al desktop su ConOut).
fn log(s: &str) {
    for b in s.bytes() {
        if b == b'\n' {
            uart(b'\r');
        }
        uart(b);
    }
}

fn uart(c: u8) {
    unsafe {
        while core::ptr::read_volatile(0x0900_0018 as *const u32) & (1 << 5) != 0 {}
        core::ptr::write_volatile(0x0900_0000 as *mut u32, c as u32);
    }
}

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
        let stall = *(bs.add(0xF8) as *const Stall);
        let n = gfx::APPS.len();
        let mut v = View { sel: Some(0), open: None };
        let mut calc = Calc::new();
        let mut term = Term::new();
        let mut out = [0u8; 24];
        let mut prompt = [0u8; 2 + term::MAX];
        gfx::draw_desktop(&fb, v, ["", ""]);
        log("[ok] desktop: frecce con w a s d, Invio apre, x chiude, q esce\n");
        loop {
            match input::read_key(st) {
                Some(k) => {
                    let ch = k.unicode as u8;
                    let sel = v.sel.unwrap_or(0);
                    match v.open {
                        None => match ch {
                            b'd' => v.sel = Some((sel + 1) % n),
                            b'a' => v.sel = Some((sel + n - 1) % n),
                            b's' => v.sel = Some((sel + 3) % n),
                            b'w' => v.sel = Some((sel + n - 3) % n),
                            b'\r' => {
                                v.open = Some(sel);
                                calc = Calc::new();
                                term = Term::new();
                                log("[ok] app aperta: ");
                                log(gfx::APPS[sel]);
                                log("\n");
                            }
                            b'q' => {
                                log("[ok] uscita\n");
                                break;
                            }
                            _ => {}
                        },
                        Some(app) if app == gfx::CALC_INDEX => match ch {
                            b'0'..=b'9' => calc.digit(ch - b'0'),
                            b'+' => calc.op(Op::Add),
                            b'-' => calc.op(Op::Sub),
                            b'*' => calc.op(Op::Mul),
                            b'/' => calc.op(Op::Div),
                            b'=' | b'\r' => {
                                calc.equals();
                                let len = calc.format(&mut out);
                                log("[ok] CALC = ");
                                log(core::str::from_utf8(&out[..len]).unwrap_or("?"));
                                log("\n");
                            }
                            b'c' => calc = Calc::new(),
                            b'x' => {
                                v.open = None;
                                log("[ok] app chiusa\n");
                            }
                            _ => {}
                        },
                        Some(app) if app == gfx::TERM_INDEX => {
                            if term.key(ch) {
                                log("[ok] TERM: ");
                                log(term.output());
                                log("\n");
                            }
                            if term.exit {
                                v.open = None;
                                log("[ok] app chiusa\n");
                            }
                        }
                        Some(_) => {
                            if ch == b'x' {
                                v.open = None;
                                log("[ok] app chiusa\n");
                            }
                        }
                    }
                    let clen = calc.format(&mut out);
                    let calc_txt = core::str::from_utf8(&out[..clen]).unwrap_or("");
                    prompt[0] = b':';
                    prompt[1] = b' ';
                    let pl = term.line().len();
                    prompt[2..2 + pl].copy_from_slice(term.line().as_bytes());
                    let prompt_txt = core::str::from_utf8(&prompt[..2 + pl]).unwrap_or("");
                    let body = if v.open == Some(gfx::TERM_INDEX) {
                        [term.output(), prompt_txt]
                    } else {
                        [calc_txt, ""]
                    };
                    gfx::draw_desktop(&fb, v, body);
                }
                None => {
                    stall(10_000);
                }
            }
        }
    }
    0 // EFI_SUCCESS
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
