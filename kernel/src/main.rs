//! Kernel di HaikuOS. Oggi: avvio via GRUB (Multiboot2) in modalità protetta
//! a 32 bit, poi primo avvio via seriale COM1.
#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::panic::PanicInfo;
use haiku_setup::wizard::{self, Console};

mod mem;

// Intestazione Multiboot2 e punto di ingresso. Lo stack è di 16 KiB.
global_asm!(
    ".section .multiboot2, \"a\"",
    ".align 8",
    "mb2_start:",
    ".long 0xe85250d6",
    ".long 0",
    ".long mb2_end - mb2_start",
    ".long -(0xe85250d6 + 0 + (mb2_end - mb2_start))",
    ".short 0",
    ".short 0",
    ".long 8",
    "mb2_end:",
    ".section .bss",
    ".align 16",
    "stack_bottom:",
    ".skip 16384",
    "stack_top:",
    ".section .text",
    ".global _start",
    "_start:",
    "mov esp, offset stack_top",
    // Abilita SSE (il codice generato lo usa): CR0.EM=0, CR0.MP=1, CR4.OSFXSR=1, CR4.OSXMMEXCPT=1.
    "mov eax, cr0",
    "and eax, 0xFFFFFFFB",
    "or eax, 0x2",
    "mov cr0, eax",
    "mov eax, cr4",
    "or eax, 0x600",
    "mov cr4, eax",
    "call kmain",
    "cli",
    "1: hlt",
    "jmp 1b",
);

const COM1: u16 = 0x3F8;
const LSR: u16 = COM1 + 5;

/// Console su porta seriale COM1 (in QEMU: `-serial stdio`).
struct Serial;

impl Console for Serial {
    fn write(&mut self, s: &str) {
        for b in s.bytes() {
            if b == b'\n' {
                put(b'\r');
            }
            put(b);
        }
    }

    fn read_line(&mut self, buf: &mut [u8], secret: bool) -> usize {
        let mut n = 0;
        loop {
            let b = get();
            match b {
                b'\r' | b'\n' => {
                    put(b'\r');
                    put(b'\n');
                    return n;
                }
                0x08 | 0x7f => {
                    if n > 0 {
                        n -= 1;
                        put(0x08);
                        put(b' ');
                        put(0x08);
                    }
                }
                0x20..=0x7e => {
                    if n < buf.len() {
                        buf[n] = b;
                        n += 1;
                        put(if secret { b'*' } else { b });
                    }
                }
                _ => {}
            }
        }
    }
}

fn put(b: u8) {
    // SAFETY: attesa del registro di stato di COM1, poi scrittura del dato.
    unsafe {
        while inb(LSR) & 0x20 == 0 {}
        outb(COM1, b);
    }
}

fn get() -> u8 {
    // SAFETY: attesa di un byte disponibile su COM1.
    unsafe {
        while inb(LSR) & 0x01 == 0 {}
        inb(COM1)
    }
}

/// Sale casuale da RDRAND; `None` se la CPU non lo fornisce.
fn salt() -> Option<[u8; 16]> {
    let mut out = [0u8; 16];
    for chunk in out.chunks_mut(4) {
        let mut v: u32 = 0;
        let mut ok = false;
        for _ in 0..10 {
            // SAFETY: istruzione RDRAND; il risultato indica se il valore è valido.
            if unsafe { core::arch::x86::_rdrand32_step(&mut v) } == 1 {
                ok = true;
                break;
            }
        }
        if !ok {
            return None;
        }
        chunk.copy_from_slice(&v.to_le_bytes());
    }
    Some(out)
}

#[unsafe(no_mangle)]
extern "C" fn kmain() -> ! {
    let mut con = Serial;
    con.write("HaikuOS: primo avvio\n");
    match salt() {
        None => con.write("Errore: RDRAND non disponibile, impossibile creare il sale.\n"),
        Some(s) => {
            let setup = wizard::run(&mut con, &s);
            con.write("--- configurazione ---\n");
            setup.render(&mut con);
            con.write("Primo avvio completato.\n");
        }
    }
    halt()
}

fn halt() -> ! {
    loop {
        // SAFETY: HLT sospende la CPU fino al prossimo interrupt.
        unsafe { asm!("hlt") };
    }
}

unsafe fn outb(port: u16, value: u8) {
    unsafe { asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags)) };
}

unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe { asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags)) };
    value
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    let mut con = Serial;
    con.write("KERNEL PANIC\n");
    halt()
}
