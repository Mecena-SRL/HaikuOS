#![no_std]
#![no_main]

#[macro_use]
mod console;
mod ata;
mod fb;
mod input;
mod rtc;
mod boot;
mod docs;
mod fs;
mod mem;
mod io;

use core::arch::asm;
use core::panic::PanicInfo;
use haiku_gui::render::Canvas;



/// Schermata di avvio: titolo, testo di stato e barra di avanzamento (pct 0..=100).
fn splash(f: &fb::Fb, pct: i32, what: &str) {
    use haiku_gui::layout::{Rect, FONT_W};
    use haiku_gui::palette::Tok::{T0, T1, T2, T3};
    use haiku_gui::render::Canvas;
    let mut c = f.canvas(0);
    let (w, h) = (f.w / f.scale, f.h / f.scale);
    let (cx, cy) = (w / 2, h / 2);
    c.fill(Rect::new(0, 0, w, h), T0);
    let cw = FONT_W + 1;
    c.text(cx - 7 * cw / 2, cy - 14, "HAIKUOS", T3);
    c.text(cx - what.len() as i32 * cw / 2, cy + 2, what, T1);
    let bw = w / 3;
    let (bx, by) = (cx - bw / 2, cy + 12);
    c.fill(Rect::new(bx - 1, by - 1, bw + 2, 6), T1);
    c.fill(Rect::new(bx, by, bw, 4), T0);
    c.fill(Rect::new(bx, by, bw * pct / 100, 4), T2);
    f.flush();
}

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    console::init();
    kprintln!("HaikuOS 0.1 - kernel x86-64 avviato in long mode");
    kprintln!("[ok] seriale COM1 e VGA attivi");

    let fbo = fb::init();
    match &fbo {
        Some(f) => {
            console::puts("[ok] framebuffer ");
            console::put_dec(f.w as u32);
            console::puts("x");
            console::put_dec(f.h as u32);
            console::puts(" bpp ");
            console::put_dec(f.bpp() as u32);
            console::puts(" pitch ");
            console::put_dec(f.pitch() as u32);
            console::puts("\n");
        }
        None => kprintln!("[avviso] framebuffer non valido: resto a testo"),
    }
    if let Some(f) = &fbo {
        splash(f, 0, "AVVIO IN CORSO");
    }

    let mut mbr = [0u8; 512];
    let disk = ata::read_sector(0, &mut mbr) && mbr[510] == 0x55 && mbr[511] == 0xAA;
    if disk {
        kprintln!("[ok] disco ATA: settore 0 letto, firma 0x55AA presente");
    } else {
        kprintln!("[errore] disco ATA: settore 0 non letto");
    }

    if !disk {
        kprintln!("[avviso] nessun disco: uso dati incorporati");
    } else if fs::show_root() {
        kprintln!("[ok] file system HAIKUFS1 letto");
    } else {
        kprintln!("[errore] file system HAIKUFS1 non trovato");
    }
    if let Some(f) = &fbo {
        splash(f, 30, if disk { "LETTURA DISCO" } else { "NESSUN DISCO: DATI INCORPORATI" });
    }

    console::puts("[ok] RAM utilizzabile: ");
    console::put_dec((mem::usable() >> 20) as u32);
    console::puts(" MB\n");
    match fbo {
        Some(f) => {
            kprintln!("[ok] framebuffer VBE con desktop");
            let mut ps2 = input::Ps2::init();
            let (lw, lh) = (f.w / f.scale, f.h / f.scale);
            let mut shell = haiku_gui::Shell::new(&ICONS, lw, lh);
            splash(&f, 60, "CARICO DOCUMENTI");
            shell.docs = docs::load();
            let files = if disk { fs::list() } else { &[] };
            if !files.is_empty() {
                shell.files_list = files;
            }
            splash(&f, 100, "PRONTO");
            let (mut cx, mut cy) = (lw / 2, lh / 2); // cursore in unità logiche
            let mut left_prev = false;
            let mut acc = (0i32, 0i32);
            let mut draw_now = true;
            let mut clock = (u8::MAX, u8::MAX);
            loop {
                let mut dirty = draw_now;
                draw_now = false;
                let now = rtc::time();
                if now != clock {
                    clock = now;
                    shell.set_clock(now.0, now.1);
                    dirty = true;
                }
                match ps2.poll() {
                    Some(input::Input::Key { key, ch }) => {
                        if let Some(k) = key {
                            shell.key(k);
                        }
                        if ch != 0 {
                            shell.text(ch);
                        }
                        dirty = true;
                    }
                    Some(input::Input::Theme) => {
                        shell.next_theme();
                        dirty = true;
                    }
                    Some(input::Input::Super) => {
                        shell.toggle_menu();
                        dirty = true;
                    }
                    Some(input::Input::Mouse { dx, dy, left }) => {
                        acc.0 += dx;
                        acc.1 -= dy;
                        cx = (cx + acc.0 / 2).clamp(0, lw - 1);
                        cy = (cy + acc.1 / 2).clamp(0, lh - 1);
                        acc = (acc.0 % 2, acc.1 % 2);
                        let left_prev_frame = left_prev;
                        if left && !left_prev {
                            shell.press(cx, cy);
                        } else if left {
                            shell.drag(cx, cy);
                        } else if left_prev {
                            shell.release();
                        }
                        left_prev = left;
                        // Ridisegna solo se il cursore o un pulsante è cambiato davvero.
                        dirty |= dx != 0 || dy != 0 || left != left_prev_frame;
                    }
                    None => {}
                }
                if dirty {
                    let mut c = f.canvas(shell.theme as usize);
                    haiku_gui::render::draw(&mut c, &shell);
                    c.fill(haiku_gui::layout::Rect::new(cx, cy, 2, 2), haiku_gui::Tok::T1);
                    f.flush();
                }
            }
        }
        None => kprintln!("[avviso] nessun framebuffer VBE, resto a testo"),
    }

    kprintln!("Sistema fermo.");
    halt()
}

// App di sistema: FILE, terminale, calcolatrice, browser, editor, impostazioni, orologio, lettore. Titoli in maiuscole per il font 3×5.
static ICONS: [haiku_gui::Icon; 11] = [
    haiku_gui::Icon { label: "FILE", app: 1, title: "FILE", w: 120, h: 80 },
    haiku_gui::Icon { label: "TERM", app: 2, title: "TERMINALE", w: 120, h: 80 },
    haiku_gui::Icon { label: "CALC", app: 3, title: "CALCOLATRICE", w: 80, h: 80 },
    haiku_gui::Icon { label: "WEB", app: 4, title: "BROWSER", w: 120, h: 80 },
    haiku_gui::Icon { label: "EDIT", app: 5, title: "EDITOR", w: 120, h: 80 },
    haiku_gui::Icon { label: "SET", app: 6, title: "IMPOST", w: 120, h: 80 },
    haiku_gui::Icon { label: "ORA", app: 7, title: "ORA", w: 80, h: 80 },
    haiku_gui::Icon { label: "READ", app: 8, title: "LETTORE", w: 120, h: 80 },
    haiku_gui::Icon { label: "IDE", app: 9, title: "IDE", w: 160, h: 120 },
    haiku_gui::Icon { label: "DOC", app: 10, title: "DOCUMENTI", w: 120, h: 80 },
    haiku_gui::Icon { label: "SCACCHI", app: 11, title: "SCACCHI", w: 120, h: 100 },
];

fn halt() -> ! {
    loop {
        unsafe { asm!("hlt", options(nomem, nostack)) };
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    kprintln!("[PANIC] kernel panic");
    halt()
}
