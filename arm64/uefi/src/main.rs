//! Applicazione UEFI di HaikuOS (ARM64): trova il GOP, disegna il desktop e stampa su ConOut.
#![no_std]
#![no_main]

mod calc;
mod docs;
mod editor;
mod fs;
mod net;
mod gfx;
mod rtc;
mod term;
mod input;

use core::ffi::c_void;
use core::panic::PanicInfo;
use calc::{Calc, Op};
use editor::Editor;
use fs::{Entry, Fs};
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

// EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_GUID {964e5b22-6459-11d2-8e39-00a0c969723b}
const SFS_GUID: Guid = Guid {
    d1: 0x964e_5b22,
    d2: 0x6459,
    d3: 0x11d2,
    d4: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

// EFI_SIMPLE_NETWORK_PROTOCOL_GUID {a19832b9-ac25-11d3-9a2d-0090273fc14d}
const SNP_GUID: Guid = Guid {
    d1: 0xa198_32b9,
    d2: 0xac25,
    d3: 0x11d3,
    d4: [0x9a, 0x2d, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d],
};

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
        let mut sfs: *mut c_void = core::ptr::null_mut();
        let fs = if locate(&SFS_GUID, core::ptr::null_mut(), &mut sfs) == 0 { Fs::open(sfs) } else { None };
        let mut st_app = Apps::new(fs);
        gfx::draw_desktop(&fb, v, &[""], st_app.theme);
        // Dopo il primo disegno: l'inizializzazione SNP può essere lenta in emulazione.
        st_app.nic = net::detect(|out| locate(&SNP_GUID, core::ptr::null_mut(), out));
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
                                st_app.open(sel);
                                if sel == gfx::FILE_INDEX {
                                    st_app.refresh();
                                }
                                log("[ok] app aperta: ");
                                log(gfx::APPS[sel]);
                                log("\n");
                                if sel == gfx::WEB_INDEX {
                                    match st_app.nic {
                                        Some(n) => {
                                            let mut b = [0u8; 17];
                                            log("[ok] RETE: MAC ");
                                            log(net::mac_str(&n.mac, &mut b));
                                            log("\n");
                                        }
                                        None => log("[ok] RETE: nessuna scheda\n"),
                                    }
                                }
                                if sel == gfx::CLOCK_INDEX {
                                    let t = rtc::hms(rtc::now());
                                    log("[ok] ORA: ");
                                    log(core::str::from_utf8(&t).unwrap_or("?"));
                                    log(" UTC\n");
                                }
                            }
                            b'q' => {
                                log("[ok] uscita\n");
                                break;
                            }
                            _ => {}
                        },
                        Some(app) => {
                            let close = match app {
                                gfx::CALC_INDEX => st_app.calc_key(ch),
                                gfx::TERM_INDEX => st_app.term_key(ch),
                                gfx::SET_INDEX => st_app.set_key(ch),
                                gfx::FILE_INDEX => st_app.file_key(ch, &mut v),
                                gfx::READ_INDEX | gfx::DOC_INDEX => st_app.reader_key(ch),
                                gfx::EDIT_INDEX => st_app.edit_key(ch),
                                _ => ch == b'x',
                            };
                            if close {
                                v.open = None;
                                log("[ok] app chiusa\n");
                            }
                        }
                    }
                    draw_all(&fb, v, &st_app);
                }
                None => {
                    if v.open == Some(gfx::CLOCK_INDEX) && rtc::now() != st_app.last_tick {
                        st_app.last_tick = rtc::now();
                        draw_all(&fb, v, &st_app);
                    }
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

/// Stato delle app: una istanza per ciascuna, conservata finché HaikuOS è in esecuzione.
struct Apps {
    calc: Calc,
    term: Term,
    ed: Editor,
    fs: Option<Fs>,
    nic: Option<net::Nic>,
    entries: [Entry; 8],
    n_entries: usize,
    file_sel: usize,
    file_name: Entry,
    file_buf: [u8; 2048],
    file_len: usize,
    doc: usize,
    scroll: usize,
    theme: usize,
    last_tick: u32,
}

const THEME_NAMES: [&str; 3] = ["TEMA 0", "TEMA 1", "TEMA 2"];
const WEB_LINES: [&str; 2] = ["WEB", "NESSUNA RETE"];

impl Apps {
    fn new(fs: Option<Fs>) -> Self {
        Apps {
            calc: Calc::new(),
            term: Term::new(),
            ed: Editor::new(),
            fs,
            nic: None,
            entries: [Entry::EMPTY; 8],
            n_entries: 0,
            file_sel: 0,
            file_name: Entry::EMPTY,
            file_buf: [0; 2048],
            file_len: 0,
            doc: 0,
            scroll: 0,
            theme: 0,
            last_tick: 0,
        }
    }

    /// Rilegge l'elenco dei file della radice.
    fn refresh(&mut self) {
        self.n_entries = match &self.fs {
            Some(fs) => fs.list(&mut self.entries),
            None => 0,
        };
        self.file_sel = self.file_sel.min(self.n_entries.saturating_sub(1));
    }

    /// Testo del file aperto (vuoto se nessun file).
    fn file_text(&self) -> &str {
        core::str::from_utf8(&self.file_buf[..self.file_len]).unwrap_or("?")
    }

    /// Apertura dall'icona `sel`: azzera lo stato dell'app.
    fn open(&mut self, sel: usize) {
        match sel {
            gfx::CALC_INDEX => self.calc = Calc::new(),
            gfx::TERM_INDEX => self.term = Term::new(),
            gfx::EDIT_INDEX => self.ed = Editor::new(),
            gfx::DOC_INDEX => {
                self.doc = 1;
                self.scroll = 0;
            }
            gfx::READ_INDEX => {
                self.file_name = Entry::EMPTY;
                self.file_len = 0;
                self.scroll = 0;
            }
            _ => {}
        }
    }

    fn calc_key(&mut self, ch: u8) -> bool {
        match ch {
            b'0'..=b'9' => self.calc.digit(ch - b'0'),
            b'+' => self.calc.op(Op::Add),
            b'-' => self.calc.op(Op::Sub),
            b'*' => self.calc.op(Op::Mul),
            b'/' => self.calc.op(Op::Div),
            b'=' | b'\r' => {
                self.calc.equals();
                let mut out = [0u8; 24];
                let len = self.calc.format(&mut out);
                log("[ok] CALC = ");
                log(core::str::from_utf8(&out[..len]).unwrap_or("?"));
                log("\n");
            }
            b'c' => self.calc = Calc::new(),
            b'x' => return true,
            _ => {}
        }
        false
    }

    fn term_key(&mut self, ch: u8) -> bool {
        if self.term.key(ch) {
            log("[ok] TERM: ");
            log(self.term.output());
            log("\n");
        }
        self.term.exit
    }

    fn set_key(&mut self, ch: u8) -> bool {
        match ch {
            b'\r' => {
                self.theme = (self.theme + 1) % 3;
                log("[ok] SET: ");
                log(THEME_NAMES[self.theme]);
                log("\n");
            }
            b'x' => return true,
            _ => {}
        }
        false
    }

    fn file_key(&mut self, ch: u8, v: &mut View) -> bool {
        let n = self.n_entries;
        match ch {
            b'w' if n > 0 => self.file_sel = (self.file_sel + n - 1) % n,
            b's' if n > 0 => self.file_sel = (self.file_sel + 1) % n,
            b'\r' if n > 0 => {
                let e = self.entries[self.file_sel];
                self.file_len = match &self.fs {
                    Some(fs) => fs.read(&e.name[..e.len], &mut self.file_buf),
                    None => 0,
                };
                self.file_name = e;
                self.scroll = 0;
                v.open = Some(gfx::READ_INDEX);
                log("[ok] READ: ");
                log(e.as_str());
                log("\n");
            }
            b'x' => return true,
            _ => {}
        }
        false
    }

    fn reader_key(&mut self, ch: u8) -> bool {
        let len = if self.file_name.len > 0 {
            self.file_text().split('\n').count()
        } else {
            docs::TEXTS[self.doc].len()
        };
        match ch {
            b'w' => self.scroll = self.scroll.saturating_sub(1),
            b's' => {
                if self.scroll + 1 < len {
                    self.scroll += 1;
                }
            }
            b'x' => return true,
            _ => {}
        }
        false
    }

    fn edit_key(&mut self, ch: u8) -> bool {
        if ch == 0x13 {
            let ok = match &self.fs {
                Some(fs) => fs.write(b"NOTA.TXT", self.ed.text().as_bytes()),
                None => false,
            };
            log(if ok { "[ok] EDIT: salvato NOTA.TXT\n" } else { "[errore] EDIT: salvataggio\n" });
            return false;
        }
        if ch == 0x11 {
            log("[ok] EDIT: ");
            log(self.ed.text());
            log("\n");
            return true;
        }
        self.ed.key(ch);
        false
    }
}

/// Disegna il desktop con lo stato di tutte le app.
fn draw_all(fb: &Fb, v: View, a: &Apps) {
    let mut out = [0u8; 24];
    let clen = a.calc.format(&mut out);
    let calc_txt = core::str::from_utf8(&out[..clen]).unwrap_or("");
    let mut prompt = [0u8; 2 + term::MAX];
    prompt[0] = b':';
    prompt[1] = b' ';
    let pl = a.term.line().len();
    prompt[2..2 + pl].copy_from_slice(a.term.line().as_bytes());
    let prompt_txt = core::str::from_utf8(&prompt[..2 + pl]).unwrap_or("");
    let clock = rtc::hms(rtc::now());
    let clock_txt = core::str::from_utf8(&clock).unwrap_or("");
    let mut ed_lines: [&str; 10] = [""; 10];
    let mut web: [&str; 5] = [""; 5];
    let mut web_mac = [0u8; 21];
    let mut rl: [&str; 10] = [""; 10];
    let mut rows: [&str; 1] = [""];
    let body: &[&str] = match v.open {
        None => &rows,
        Some(gfx::CALC_INDEX) => {
            rows[0] = calc_txt;
            &rows
        }
        Some(gfx::TERM_INDEX) => {
            let t = [a.term.output(), prompt_txt];
            return gfx::draw_desktop(fb, v, &t, a.theme);
        }
        Some(gfx::CLOCK_INDEX) => {
            rows[0] = clock_txt;
            &rows
        }
        Some(gfx::SET_INDEX) => {
            let t = [THEME_NAMES[a.theme], "INVIO CAMBIA"];
            return gfx::draw_desktop(fb, v, &t, a.theme);
        }
        Some(gfx::FILE_INDEX) => {
            let mut rows = [[0u8; 16]; 8];
            let mut k = 0;
            for (r, row) in rows.iter_mut().enumerate().take(a.n_entries) {
                let name = a.entries[r].as_str().as_bytes();
                row[0] = if r == a.file_sel { b'>' } else { b' ' };
                row[1] = b' ';
                row[2..2 + name.len()].copy_from_slice(name);
                k = 2 + name.len();
            }
            let _ = k;
            let mut lines: [&str; 8] = [""; 8];
            for (r, row) in rows.iter().enumerate().take(a.n_entries) {
                let end = row.iter().position(|&b| b == 0).unwrap_or(row.len());
                lines[r] = core::str::from_utf8(&row[..end]).unwrap_or("");
            }
            if a.n_entries == 0 {
                lines[0] = "NESSUN FILE";
                return gfx::draw_desktop(fb, v, &lines[..1], a.theme);
            }
            return gfx::draw_desktop(fb, v, &lines[..a.n_entries], a.theme);
        }
        Some(gfx::READ_INDEX) if a.file_name.len == 0 => &["APRI UN FILE", "DAL MENU FILE"],
        Some(gfx::READ_INDEX) => {
            let mut k = 0;
            for line in a.file_text().split('\n').skip(a.scroll).take(10) {
                rl[k] = line;
                k += 1;
            }
            &rl[..k]
        }
        Some(gfx::DOC_INDEX) => &docs::TEXTS[1][a.scroll..],
        Some(gfx::EDIT_INDEX) => {
            let k = a.ed.last_lines(&mut ed_lines, 12);
            &ed_lines[..k]
        }
        Some(gfx::WEB_INDEX) => match a.nic {
            None => &WEB_LINES,
            Some(n) => {
                let mut mb = [0u8; 17];
                let m = net::mac_str(&n.mac, &mut mb);
                web_mac[..4].copy_from_slice(b"MAC ");
                web_mac[4..21].copy_from_slice(m.as_bytes());
                web[0] = "WEB";
                web[1] = "SCHEDA: SNP";
                web[2] = core::str::from_utf8(&web_mac).unwrap_or("");
                web[3] = if n.up { "ENLACE SU" } else { "ENLACE GIU" };
                web[4] = "HTTP DA FARE";
                &web
            }
        },
        Some(_) => &rows,
    };
    gfx::draw_desktop(fb, v, body, a.theme);
}
