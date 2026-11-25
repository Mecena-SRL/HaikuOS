//! HaikuOS su GBA: avvio, stato del sistema, misure, spazio di lavoro.
//! Nessun bootloader separato: questa ROM è il primo stadio.
#![no_std]
#![no_main]

mod alloc;
mod calc;
mod calc_ui;
mod compositor;
mod desktop;
mod font;
mod fs;
mod chess;
mod editor;
#[allow(dead_code)]
mod media_ui;
mod glyph;
mod monolith;
mod persist;
mod surfaces;
mod space;
mod sram;
mod workspace;

use core::arch::global_asm;
use core::cell::RefCell;
use core::panic::PanicInfo;
use core::ptr::{read_volatile, write_volatile};
use fs::FILES;
use monolith::{Monolith, AGENT_BYTES, MAX_PROGRAMS, MONOLITH_BYTES, RAM_BYTES, SLOT_BYTES};
use workspace::{child_name, Action, Workspace};

/// Tag letto da mGBA: indica una SRAM da 32 KB per il salvataggio.
#[used]
static SAVE_TAG: [u8; 12] = *b"SRAM_V113\0\0\0";

// Header ROM (0x00-0xBF) e avvio in ARM. Logo Nintendo e checksum NON inclusi (test solo in emulatore).
global_asm!(
    ".section .text.start,\"ax\",%progbits",
    ".code 32",
    ".global _start",
    "_start:",
    "    b 2f",
    "    .space 0xA0 - 4, 0",
    "    .ascii \"HAIKUOS\"",
    "    .space 12 - 7, 0",
    "    .ascii \"HKOS\"",
    "    .ascii \"00\"",
    "    .byte 0x96, 0",
    "    .space 12, 0",
    "2:",
    "    ldr sp, =0x03007F00",
    "    ldr r0, =__sidata",
    "    ldr r1, =__sdata",
    "    ldr r2, =__edata",
    "3:  cmp r1, r2",
    "    ldrlo r3, [r0], #4",
    "    strlo r3, [r1], #4",
    "    blo 3b",
    "    ldr r1, =__sbss",
    "    ldr r2, =__ebss",
    "    mov r3, #0",
    "4:  cmp r1, r2",
    "    strlo r3, [r1], #4",
    "    blo 4b",
    "    ldr r0, =rust_main",
    "    orr r0, r0, #1",
    "    bx r0",
);

const VRAM: usize = 0x0600_0000;
const DISPCNT: *mut u16 = 0x0400_0000 as *mut u16;
const KEYINPUT: *const u16 = 0x0400_0130 as *const u16;
const TM0: usize = 0x0400_0100;
const TM1: usize = 0x0400_0104;
const W: usize = 240;

const fn rgb(r: u16, g: u16, b: u16) -> u16 {
    r | (g << 5) | (b << 10)
}
const BLACK: u16 = rgb(0, 0, 0);
const GREEN: u16 = rgb(4, 24, 6);
const BLUE: u16 = rgb(6, 14, 28);
const ORANGE: u16 = rgb(30, 16, 2);
const GREY: u16 = rgb(10, 10, 10);
const DARK: u16 = rgb(3, 3, 4);
const WHITE: u16 = rgb(31, 31, 31);
const RED: u16 = rgb(31, 0, 0);


fn fill(x: usize, y: usize, w: usize, h: usize, c: u16) {
    for yy in y..y + h {
        for xx in x..x + w {
            // Mode 3: framebuffer 240x160 a 16 bit.
            unsafe { write_volatile((VRAM as *mut u16).add(yy * W + xx), c) };
        }
    }
}

/// Barra della memoria: lunghezze proporzionali al pool totale (216 px).
fn draw_memory(granted: &[usize; MAX_PROGRAMS]) {
    const BAR_W: usize = 216;
    let px = |bytes: usize| bytes * BAR_W / RAM_BYTES;
    let mut x = 12;
    for (bytes, c) in [(MONOLITH_BYTES, GREEN), (AGENT_BYTES, BLUE)] {
        let w = px(bytes);
        fill(x, 30, w, 24, c);
        x += w;
    }
    for &b in granted.iter().filter(|&&b| b != 0) {
        let w = px(b);
        fill(x, 30, w, 24, ORANGE);
        x += w;
    }
    fill(x, 30, 12 + BAR_W - x, 24, DARK);
}

/// Quattro slot: arancio se assegnato, grigio se libero.
fn draw_slots(granted: &[usize; MAX_PROGRAMS]) {
    for (i, &b) in granted.iter().enumerate() {
        let c = if b != 0 { ORANGE } else { GREY };
        fill(12 + i * 56, 80, 40, 40, c);
    }
}

/// Un valore a 32 bit come riga di 32 quadretti (1 = colorato). Bit alto a sinistra.
fn draw_bits(y: usize, v: u32, c: u16) {
    for i in 0..32 {
        if v & (1 << (31 - i)) != 0 {
            fill(4 + i * 7, y, 6, 8, c);
        }
    }
}

/// Cicli (prescaler 1) impiegati da `f`, con TM0+TM1 in cascata (32 bit).
fn cycles<F: FnOnce()>(f: F) -> u32 {
    unsafe {
        write_volatile(TM0 as *mut u16, 0);
        write_volatile((TM0 + 2) as *mut u16, 0);
        write_volatile(TM1 as *mut u16, 0);
        write_volatile((TM1 + 2) as *mut u16, 0);
        write_volatile((TM0 + 2) as *mut u16, 0x0080);
        write_volatile((TM1 + 2) as *mut u16, 0x0084);
    }
    f();
    let t = now();
    unsafe {
        write_volatile((TM0 + 2) as *mut u16, 0);
        write_volatile((TM1 + 2) as *mut u16, 0);
    }
    t
}

fn now() -> u32 {
    loop {
        let hi1 = unsafe { read_volatile(TM1 as *const u16) };
        let lo = unsafe { read_volatile(TM0 as *const u16) };
        let hi2 = unsafe { read_volatile(TM1 as *const u16) };
        if hi1 == hi2 {
            return ((hi1 as u32) << 16) | lo as u32;
        }
    }
}

/// Tasti premuti (1 = premuto). Il registro è attivo basso.
fn keys() -> u16 {
    !unsafe { read_volatile(KEYINPUT) } & 0x03FF
}

fn draw_workspace(ws: &Workspace, p: &persist::Persist<space::Window<'_, sram::Sram>>) {
    fill(0, 0, W, 160, BLACK);
    // Barra della directory corrente: lunghezza = percorso.
    let bar = (ws.cwd().len() * 8 + 8).min(232);
    fill(4, 4, bar, 8, BLUE);
    let mut ids = [0usize; FILES];
    let n = p.fs().list(ws.cwd(), &mut ids);
    for i in 0..n {
        let x = 8 + (i % 7) * 30;
        let y = 24 + (i / 7) * 30;
        if i == ws.sel() {
            fill(x - 2, y - 2, 26, 26, WHITE);
        }
        let c = if p.fs().is_dir(ids[i]) { BLUE } else { ORANGE };
        fill(x, y, 22, 22, c);
    }
    // File aperto: una barra verde proporzionale alla lunghezza.
    if let Some(id) = ws.open_id() {
        let len = p.fs().len_of(id).unwrap_or(0);
        fill(4, 140, (len * 2).min(232), 8, GREEN);
    }
    // Conferma formattazione: fascia rossa, A = formatta (verde), B = annulla (grigio).
    if ws.confirming() {
        fill(0, 100, W, 40, RED);
        fill(20, 110, 60, 20, GREEN);
        fill(W - 80, 110, 60, 20, WHITE);
    }
}

/// Ridisegna il visualizzatore: testo verde, carattere in attesa arancio, riga di stato blu,
/// poi compone in VRAM. `status`: messaggio dopo un salvataggio.
fn draw_editor(surf: &mut surfaces::Surfaces, ed: &editor::Editor, status: Option<&[u8]>) {
    if let Some(px) = surf.pixels_mut(0) {
        px.fill(BLACK);
        glyph::draw_wrapped(px, surfaces::SW, surfaces::SH, ed.text(), ed.pending(), GREEN, ORANGE);
        if let Some(msg) = status {
            glyph::draw_text(px, surfaces::SW, surfaces::SH, 2, 40, msg, BLUE);
        }
    }
    let fb = unsafe { core::slice::from_raw_parts_mut(VRAM as *mut u16, W * 160) };
    surf.compose_into(fb);
}

/// Editor a schermo intero della superficie 0 (il programma del visualizzatore).
/// Su/Giù/A/B modificano la riga; Select chiama `save`; Start esce e chiude la superficie.
fn run_editor(surf: &mut surfaces::Surfaces, m: &mut Monolith, text: &[u8], save: &mut dyn FnMut(&[u8]) -> bool) {
    let _ = surf.open(m, 0, 10, 20);
    let mut ed = editor::Editor::new();
    ed.load(text);
    draw_editor(surf, &ed, None);
    let mut prev = keys();
    loop {
        sram::vblank();
        let pressed = keys() & !prev;
        prev = keys();
        if pressed & workspace::KEY_START != 0 {
            break;
        }
        if pressed & workspace::KEY_SELECT != 0 {
            let msg: &[u8] = if save(ed.text()) { b"SALVATO" } else { b"ERRORE" };
            draw_editor(surf, &ed, Some(msg));
            continue;
        }
        let changed = if pressed & workspace::KEY_UP != 0 {
            ed.up();
            true
        } else if pressed & workspace::KEY_DOWN != 0 {
            ed.down();
            true
        } else if pressed & workspace::KEY_A != 0 {
            ed.commit()
        } else if pressed & workspace::KEY_B != 0 {
            ed.backspace()
        } else {
            false
        };
        if changed {
            draw_editor(surf, &ed, None);
        }
    }
    let _ = surf.close(m, 0);
}

static mut SURFACES: surfaces::Surfaces = surfaces::Surfaces::new();
/// Superficie propria della calcolatrice (in EWRAM, non nello stack).
static mut CALC_SURF: [u16; calc_ui::SW * calc_ui::SH] = [0; calc_ui::SW * calc_ui::SH];

impl surfaces::Quota for Monolith {
    fn take(&mut self, p: usize, bytes: usize) -> Option<usize> {
        self.alloc(p, bytes).ok()
    }
    fn give(&mut self, p: usize, off: usize, bytes: usize) -> bool {
        self.free(p, off, bytes)
    }
}

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    // Stato iniziale: monolite attivo, agente pronto.
    let mut m = Monolith::new();
    for p in 0..MAX_PROGRAMS {
        m.register(p);
    }
    // Sequenza di boot: 3 concessi, il quarto negato dallo slot riservato, poi revoca e riassegna.
    let _ = m.request(0, SLOT_BYTES);
    let _ = m.request(1, SLOT_BYTES);
    let _ = m.request(2, SLOT_BYTES);
    let _ = m.request(3, SLOT_BYTES);
    m.revoke(1);
    let _ = m.request(3, SLOT_BYTES);
    let alloc_ok = match m.alloc(0, 4096) {
        Ok(off) => m.free(0, off, 4096),
        Err(_) => false,
    };
    let granted = [m.grant_of(0), m.grant_of(1), m.grant_of(2), m.grant_of(3)];

    unsafe { write_volatile(DISPCNT, 0x0403) }; // mode 3, BG2 attivo
    fill(0, 0, W, 160, BLACK);
    fill(12, 12, 216, 12, GREEN); // titolo: monolite attivo
    draw_memory(&granted);
    draw_slots(&granted);
    fill(12, 130, 16, 16, GREEN); // monolite: vivo
    fill(36, 130, 16, 16, BLUE); // agente: pronto

    // Due spazi nella SRAM: sistema (contatore di avvii) e utente (file e directory).
    let dev = RefCell::new(sram::Sram);
    let mut sp = space::Spaces::open(&dev, sram::SIZE / 2, sram::SIZE / 2);
    let mut b = [0u8; 1];
    let found = sp.sys.read(b"boots", space::SYS_CAP, &mut b) == Ok(1);
    if !found {
        let _ = sp.sys.create(b"boots", space::SYS_CAP);
    }
    let boots = if found { b[0].wrapping_add(1) } else { 1 };
    let ok = sp.sys.write(b"boots", space::SYS_CAP, 1, &[boots]).is_ok() && sp.sys.sync().is_ok();
    let _ = sp.usr.mkdir(b"docs", 0);
    // Il file di esempio si crea solo al primo avvio: non sovrascrive il testo salvato.
    if sp.usr.read(b"docs/hello", 0, &mut [0u8; 1]).is_err() {
        let _ = sp.usr.create(b"docs/hello", 0);
        let _ = sp.usr.write(b"docs/hello", 0, 64, b"hello haikuos");
    }
    fill(60, 130, 16, 16, if ok { GREEN } else { ORANGE }); // persistenza: ok
    fill(84, 130, 16, 16, if alloc_ok { GREEN } else { ORANGE }); // allocatore: ok
    for i in 0..(boots as usize).min(8) {
        fill(12 + i * 20, 146, 12, 10, BLUE); // avvii salvati
    }
    for _ in 0..180 {
        sram::vblank();
    }

    // Misure (cicli di timer, emulatore mGBA): media di alloc+free e di sync con 100 byte.
    let _ = sp.usr.create(b"bench", 0);
    let mut sum_alloc = 0u32;
    for _ in 0..100 {
        sum_alloc = sum_alloc.wrapping_add(cycles(|| {
            if let Ok(off) = m.alloc(0, 4096) {
                m.free(0, off, 4096);
            }
        }));
    }
    let data = [0xA5u8; 100];
    let mut sum_sync = 0u32;
    for _ in 0..10 {
        let _ = sp.usr.write(b"bench", 0, 128, &data);
        sum_sync = sum_sync.wrapping_add(cycles(|| {
            let _ = sp.usr.sync();
        }));
    }
    fill(0, 0, W, 160, BLACK);
    draw_bits(40, sum_alloc / 100, ORANGE); // alloc+free, cicli
    draw_bits(70, sum_sync / 10, GREEN); // sync 100 byte, cicli
    // Schermata delle misure: resta finché non si preme A.
    while keys() & workspace::KEY_A == 0 {
        sram::vblank();
    }
    while keys() & workspace::KEY_A != 0 {
        sram::vblank();
    }

    // Demo: superfici dei programmi 0, 2 e 3 (quota dal monolite), composte in VRAM.
    let surf = unsafe { &mut *core::ptr::addr_of_mut!(SURFACES) };
    fill(0, 0, W, 160, BLACK);
    let _ = surf.open(&mut m, 0, 10, 20);
    let _ = surf.open(&mut m, 2, 50, 50);
    let _ = surf.open(&mut m, 3, 100, 70);
    if let Some(px) = surf.pixels_mut(2) {
        px.fill(BLUE);
    }
    if let Some(px) = surf.pixels_mut(3) {
        px.fill(ORANGE);
    }
    // Primo programma grafico: visualizzatore di testo. Legge docs/hello dal file system.
    let mut t = [0u8; editor::MAX];
    let n = sp.usr.read(b"docs/hello", 0, &mut t).unwrap_or(0);
    run_editor(surf, &mut m, &t[..n], &mut |txt| {
        sp.usr.write(b"docs/hello", 0, editor::MAX, txt).is_ok() && sp.usr.sync().is_ok()
    });
    // Chiusura: ogni programma riottiene la sua quota.
    for p in [2, 3] {
        let _ = surf.close(&mut m, p);
    }

    // Desktop del concept: croce per scegliere un'icona, A apre il terminale, B lo chiude.
    // Start passa allo spazio di lavoro.
    let mut dk = desktop::Desktop::new();
    let fb = unsafe { core::slice::from_raw_parts_mut(VRAM as *mut u16, W * 160) };
    let surf = unsafe { &mut *core::ptr::addr_of_mut!(CALC_SURF) };
    dk.draw_calc(surf);
    desktop::paint(fb, &dk, surf);
    while keys() & workspace::KEY_START == 0 {
        sram::vblank();
        if dk.step(keys()) {
            dk.draw_calc(surf);
            desktop::paint(fb, &dk, surf);
        }
    }
    while keys() & workspace::KEY_START != 0 {
        sram::vblank();
    }
    // Spazio di lavoro sullo spazio utente: tasti per navigare, Start crea un file.
    let mut ws = Workspace::new();
    loop {
        sram::vblank();
        let _ = sp.sys.frame();
        let _ = sp.usr.frame();
        match ws.step(sp.usr.fs(), keys()) {
            Action::Create => {
                let mut ids = [0usize; FILES];
                let n = sp.usr.fs().list(ws.cwd(), &mut ids) as u8;
                let (nm, nl) = child_name(ws.cwd(), n);
                if nl > 0 && sp.usr.create(&nm[..nl], 0).is_ok() {
                    let _ = sp.usr.write(&nm[..nl], 0, 64, &[n; 8]);
                }
                ws.dirty = true;
            }
            Action::FormatSys => {
                // Il workspace agisce con la capacità di sistema: formatta solo lo spazio di sistema.
                let _ = sp.format_system(space::SYS_CAP);
                ws.dirty = true;
            }
            Action::FormatAsk => {}
            Action::File => {
                // A su un file: lo apre nel visualizzatore. Start torna allo spazio di lavoro.
                if let Some(id) = ws.open_id() {
                    let mut name = [0u8; 24];
                    let nl = sp.usr.fs().info(id).map(|(nm, _)| nm.len().min(name.len())).unwrap_or(0);
                    if let Some((nm, _)) = sp.usr.fs().info(id) {
                        name[..nl].copy_from_slice(&nm[..nl]);
                    }
                    let mut buf = [0u8; editor::MAX];
                    let n = sp.usr.read(&name[..nl], 0, &mut buf).unwrap_or(0);
                    fill(0, 0, W, 160, BLACK);
                    let surf = unsafe { &mut *core::ptr::addr_of_mut!(SURFACES) };
                    run_editor(surf, &mut m, &buf[..n], &mut |txt| {
                        sp.usr.write(&name[..nl], 0, editor::MAX, txt).is_ok() && sp.usr.sync().is_ok()
                    });
                }
                ws.dirty = true;
            }
            _ => {}
        }
        if ws.dirty {
            draw_workspace(&ws, &sp.usr);
            ws.dirty = false;
        }
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
