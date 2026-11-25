// Accesso alle porte I/O x86.
use core::arch::asm;

#[inline]
pub unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    asm!("in al, dx", out("al") v, in("dx") port, options(nomem, nostack, preserves_flags));
    v
}

#[inline]
pub unsafe fn inw(port: u16) -> u16 {
    let v: u16;
    asm!("in ax, dx", out("ax") v, in("dx") port, options(nomem, nostack, preserves_flags));
    v
}
