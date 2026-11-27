//! Kernel ARM64 di HaikuOS (primo passo): avvio su QEMU virt in EL1, seriale PL011.
#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU32, Ordering};

static TICKS: AtomicU32 = AtomicU32::new(0);

mod gic;
mod mmu;
mod timer;
mod uart;


// Tabella dei vettori EL1 (2 KB, voci da 128 byte). Solo IRQ da SP_EL1 (voce 0x280) è usata.
global_asm!(
    ".balign 2048",
    ".global vectors",
    "vectors:",
    ".balign 0x80", "b .",          // 0x000 sincrono SP0
    ".balign 0x80", "b .",          // 0x080 IRQ SP0
    ".balign 0x80", "b .",          // 0x100 FIQ SP0
    ".balign 0x80", "b .",          // 0x180 SError SP0
    ".balign 0x80", "b .",          // 0x200 sincrono SPx
    ".balign 0x80", "b irq_entry",  // 0x280 IRQ SPx
    ".balign 0x80", "b .",          // 0x300 FIQ SPx
    ".balign 0x80", "b .",          // 0x380 SError SPx
    "irq_entry:",
    "stp x0, x1, [sp, #-16]!",
    "stp x2, x3, [sp, #-16]!",
    "stp x4, x5, [sp, #-16]!",
    "stp x6, x7, [sp, #-16]!",
    "stp x8, x9, [sp, #-16]!",
    "stp x10, x11, [sp, #-16]!",
    "stp x12, x13, [sp, #-16]!",
    "stp x14, x15, [sp, #-16]!",
    "stp x16, x17, [sp, #-16]!",
    "stp x18, x30, [sp, #-16]!",
    "bl irq_handler",
    "ldp x18, x30, [sp], #16",
    "ldp x16, x17, [sp], #16",
    "ldp x14, x15, [sp], #16",
    "ldp x12, x13, [sp], #16",
    "ldp x10, x11, [sp], #16",
    "ldp x8, x9, [sp], #16",
    "ldp x6, x7, [sp], #16",
    "ldp x4, x5, [sp], #16",
    "ldp x2, x3, [sp], #16",
    "ldp x0, x1, [sp], #16",
    "eret",
);

extern "C" {
    static vectors: u8;
}

#[no_mangle]
extern "C" fn irq_handler() {
    let iar = gic::ack();
    let id = iar & 0x3FF;
    if id == timer::IRQ {
        TICKS.fetch_add(1, Ordering::Relaxed);
        timer::arm_tick(100);
    }
    if id == uart::IRQ {
        uart::read_all(|c| {
            puts("RX:");
            putc(c);
            puts("\n");
        });
        uart::clear_irq();
    }
    gic::eoi(iar);
}

fn irq_init() {
    unsafe {
        let v = core::ptr::addr_of!(vectors) as u64;
        asm!("msr vbar_el1, {}", "isb", in(reg) v);
    }
    gic::enable_irq(uart::IRQ);
    uart::rx_irq_on();
    unsafe { asm!("msr daifclr, #2") }
}

const UART_DR: *mut u32 = 0x0900_0000 as *mut u32;
const UART_FR: *const u32 = 0x0900_0018 as *const u32;
const FR_TXFF: u32 = 1 << 5;

// Ingresso: stack, azzeramento BSS, poi kmain(). Anche fuori da EL1, kmain decide.
global_asm!(
    ".section .text.boot, \"ax\"",
    ".global _start",
    "_start:",
    "adrp x0, __stack_top",
    "add x0, x0, :lo12:__stack_top",
    "mov sp, x0",
    "adrp x1, __bss_start",
    "add x1, x1, :lo12:__bss_start",
    "adrp x2, __bss_end",
    "add x2, x2, :lo12:__bss_end",
    "1:",
    "cmp x1, x2",
    "b.hs 2f",
    "str xzr, [x1], #8",
    "b 1b",
    "2:",
    "bl kmain",
    "3:",
    "wfe",
    "b 3b",
);

fn putc(c: u8) {
    unsafe {
        while core::ptr::read_volatile(UART_FR) & FR_TXFF != 0 {}
        core::ptr::write_volatile(UART_DR, c as u32);
    }
}

fn puts(s: &str) {
    for b in s.bytes() {
        if b == b'\n' {
            putc(b'\r');
        }
        putc(b);
    }
}

fn put_dec(mut n: u64) {
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    for &b in &buf[i..] {
        putc(b);
    }
}

fn halt() -> ! {
    loop {
        unsafe { asm!("wfe") }
    }
}

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    puts("HaikuOS 0.1 - kernel ARM64\n");
    let el: u64;
    unsafe { asm!("mrs {}, CurrentEL", out(reg) el) };
    // CurrentEL bit [3:2]: 1 = EL1. Altrimenti stop (nessun avvio parziale).
    if (el >> 2) & 3 != 1 {
        puts("[errore] EL diverso da EL1: fermo\n");
        halt();
    }
    puts("[ok] seriale PL011 e EL1\n");
    let ticks = timer::busy_wait_ms(100);
    puts("[ok] timer generico: ");
    put_dec(timer::freq());
    puts(" Hz, 100 ms = ");
    put_dec(ticks);
    puts(" tick\n");
    if mmu::enable() {
        puts("[ok] MMU attiva (identita, 2 GB)\n");
    } else {
        puts("[errore] MMU non attiva\n");
    }
    irq_init();
    puts("[ok] GIC e UART RX con interrupt: scrivi nel seriale\n");
    gic::enable_irq(timer::IRQ);
    timer::arm_tick(100);
    while TICKS.load(Ordering::Relaxed) < 3 {
        unsafe { asm!("wfi") }
    }
    puts("[ok] timer con interrupt: 3 tick da 100 ms\n");
    loop {
        unsafe { asm!("wfi") }
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    halt()
}
