#![no_std]
#![no_main]

#[macro_use]
mod console;
mod ata;
mod fb;
mod boot;
mod fs;
mod io;

use core::arch::asm;
use core::panic::PanicInfo;

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
            let mut c = f.canvas();
            let (lw, lh) = c.size();
            let shell = haiku_gui::Shell::new(&ICONS, lw, lh);
            haiku_gui::render::draw(&mut c, &shell);
            kprintln!("[ok] framebuffer VBE 640x480 con desktop");
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
