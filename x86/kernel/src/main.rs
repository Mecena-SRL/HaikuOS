#![no_std]
#![no_main]

#[macro_use]
mod console;
mod ata;
mod fb;
mod input;
mod boot;
mod fs;
mod io;

use core::arch::asm;
use core::panic::PanicInfo;
use haiku_gui::render::Canvas;

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    console::init();
    kprintln!("HaikuOS 0.1 - kernel x86-64 avviato in long mode");
    kprintln!("[ok] seriale COM1 e VGA attivi");

    let mut mbr = [0u8; 512];
    if ata::read_sector(0, &mut mbr) && mbr[510] == 0x55 && mbr[511] == 0xAA {
        kprintln!("[ok] disco ATA: settore 0 letto, firma 0x55AA presente");
    } else {
        kprintln!("[errore] disco ATA: settore 0 non letto");
    }

    if fs::show_root() {
        kprintln!("[ok] file system HAIKUFS1 letto");
    } else {
        kprintln!("[errore] file system HAIKUFS1 non trovato");
    }

    match fb::init() {
        Some(f) => {
            kprintln!("[ok] framebuffer VBE 640x480 con desktop");
            let mut ps2 = input::Ps2::init();
            let (lw, lh) = (f.w / fb::SCALE, f.h / fb::SCALE);
            let mut shell = haiku_gui::Shell::new(&ICONS, lw, lh);
            let (mut cx, mut cy) = (lw / 2, lh / 2); // cursore in unità logiche
            let mut left_prev = false;
            let mut acc = (0i32, 0i32);
            let mut draw_now = true;
            loop {
                let mut dirty = draw_now;
                draw_now = false;
                match ps2.poll() {
                    Some(input::Input::Key(k)) => {
                        shell.key(k);
                        dirty = true;
                    }
                    Some(input::Input::Mouse { dx, dy, left }) => {
                        acc.0 += dx;
                        acc.1 -= dy;
                        cx = (cx + acc.0 / 2).clamp(0, lw - 1);
                        cy = (cy + acc.1 / 2).clamp(0, lh - 1);
                        acc = (acc.0 % 2, acc.1 % 2);
                        if left && !left_prev {
                            shell.click(cx, cy);
                        }
                        left_prev = left;
                        dirty = true;
                    }
                    None => {}
                }
                if dirty {
                    let mut c = f.canvas();
                    haiku_gui::render::draw(&mut c, &shell);
                    c.fill(haiku_gui::layout::Rect::new(cx, cy, 2, 2), haiku_gui::Tok::T1);
                }
            }
        }
        None => kprintln!("[avviso] nessun framebuffer VBE, resto a testo"),
    }

    kprintln!("Sistema fermo.");
    halt()
}

static ICONS: [haiku_gui::Icon; 3] = [
    haiku_gui::Icon { label: "FILE", app: 1, title: "FILE", w: 120, h: 80 },
    haiku_gui::Icon { label: "TERM", app: 2, title: "TERMINALE", w: 120, h: 80 },
    haiku_gui::Icon { label: "CALC", app: 3, title: "CALCOLATRICE", w: 80, h: 80 },
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
