//! HaikuOS su GBA: prima ROM. Il monolite gira, l'agente risponde, gli slot si assegnano.
//! Nessun bootloader separato: questa ROM è il primo stadio.
#![no_std]
#![no_main]

mod fs;
mod monolith;
mod persist;
mod sram;

use core::arch::global_asm;
use core::panic::PanicInfo;
use core::ptr::write_volatile;
use monolith::{Monolith, MONOLITH_BYTES, AGENT_BYTES, POOL_BYTES, SLOT_BYTES, RAM_BYTES, MAX_PROGRAMS};

// Header ROM (0x00-0xBF) e avvio in ARM. Logo Nintendo e checksum NON inclusi:
// gli emulatori li ignorano, l'hardware reale no. Da verificare.
/// Tag letto da mGBA: indica una SRAM da 32 KB per il salvataggio.
#[used]
static SAVE_TAG: [u8; 12] = *b"SRAM_V113\0\0\0";

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
    let segs_fixed = [(MONOLITH_BYTES, GREEN), (AGENT_BYTES, BLUE)];
    for (bytes, c) in segs_fixed {
        let w = px(bytes);
        fill(x, 30, w, 24, c);
        x += w;
    }
    let mut used = 0;
    for &b in granted.iter().filter(|&&b| b != 0) {
        let w = px(b);
        fill(x, 30, w, 24, ORANGE);
        x += w;
        used += b;
    }
    let _ = (POOL_BYTES, used);
    fill(x, 30, 12 + BAR_W - x, 24, DARK);
}

/// Quattro slot: arancio se assegnato, grigio se libero.
fn draw_slots(granted: &[usize; MAX_PROGRAMS]) {
    for (i, &b) in granted.iter().enumerate() {
        let c = if b != 0 { ORANGE } else { GREY };
        fill(12 + i * 56, 80, 40, 40, c);
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
    let _ = m.request(3, SLOT_BYTES); // negato: NoFreeSlot
    m.revoke(1);
    let _ = m.request(3, SLOT_BYTES);

    let granted = [m.grant_of(0), m.grant_of(1), m.grant_of(2), m.grant_of(3)];

    unsafe { write_volatile(DISPCNT, 0x0403) }; // mode 3, BG2 attivo
    fill(0, 0, W, 160, BLACK);
    fill(12, 12, 216, 12, GREEN); // titolo: monolite attivo
    draw_memory(&granted);
    draw_slots(&granted);
    fill(12, 130, 16, 16, GREEN); // monolite: vivo
    fill(36, 130, 16, 16, BLUE); // agente: pronto
    // Persistenza in SRAM: contatore di avvii. Ogni avvio lo incrementa e lo salva.
    let mut p = persist::Persist::open(sram::Sram, (sram::SIZE - 32) / 2);
    let mut b = [0u8; 1];
    let found = p.read(b"boots", 0, &mut b) == Ok(1);
    if !found {
        let _ = p.create(b"boots", 0);
    }
    let boots = if found { b[0].wrapping_add(1) } else { 1 };
    let ok = p.write(b"boots", 0, 1, &[boots]).is_ok() && p.sync().is_ok();
    fill(60, 130, 16, 16, if ok { GREEN } else { ORANGE }); // persistenza: ok
    for i in 0..(boots as usize).min(8) {
        fill(12 + i * 20, 146, 12, 10, BLUE); // avvii salvati
    }
    loop {
        sram::vblank();
        let _ = p.frame();
    }

}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
