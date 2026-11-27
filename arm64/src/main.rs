//! Kernel ARM64 di HaikuOS (primo passo): avvio su QEMU virt in EL1, seriale PL011.
#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::panic::PanicInfo;

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
    puts("Sistema fermo.\n");
    halt()
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    halt()
}
