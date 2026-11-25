// Output su seriale COM1 e su schermo VGA testo (80x25).
use crate::io::{inb, outb};
use core::ptr::{read_volatile, write_volatile};

const COM1: u16 = 0x3F8;
const VGA: usize = 0xB8000;
const COLS: usize = 80;
const ROWS: usize = 25;
const BLANK: u16 = 0x0F20;

static mut ROW: usize = 0;
static mut COL: usize = 0;

pub fn init() {
    unsafe {
        outb(COM1 + 1, 0x00); // COM1 a 115200 baud, 8N1, FIFO attivo
        outb(COM1 + 3, 0x80);
        outb(COM1, 0x01);
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x03);
        outb(COM1 + 2, 0xC7);
        outb(COM1 + 4, 0x0B);
        for i in 0..COLS * ROWS {
            write_volatile((VGA + i * 2) as *mut u16, BLANK);
        }
        ROW = 0;
        COL = 0;
    }
}

fn serial_byte(b: u8) {
    unsafe {
        while inb(COM1 + 5) & 0x20 == 0 {}
        outb(COM1, b);
    }
}

fn vga_byte(b: u8) {
    unsafe {
        if b == b'\n' {
            return newline();
        }
        let i = ROW * COLS + COL;
        write_volatile((VGA + i * 2) as *mut u16, 0x0F00 | b as u16);
        COL += 1;
        if COL == COLS {
            newline();
        }
    }
}

unsafe fn newline() {
    COL = 0;
    if ROW + 1 < ROWS {
        ROW += 1;
        return;
    }
    for r in 1..ROWS {
        for c in 0..COLS {
            let v = read_volatile((VGA + (r * COLS + c) * 2) as *const u16);
            write_volatile((VGA + ((r - 1) * COLS + c) * 2) as *mut u16, v);
        }
    }
    for c in 0..COLS {
        write_volatile((VGA + ((ROWS - 1) * COLS + c) * 2) as *mut u16, BLANK);
    }
}

pub fn puts(s: &str) {
    for b in s.bytes() {
        if b == b'\n' {
            serial_byte(b'\r');
        }
        serial_byte(b);
        vga_byte(b);
    }
}

pub fn put_dec(mut n: u32) {
    let mut buf = [0u8; 10];
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    puts(unsafe { core::str::from_utf8_unchecked(&buf[i..]) });
}

#[macro_export]
macro_rules! kprintln {
    ($s:expr) => {{
        $crate::console::puts($s);
        $crate::console::puts("\n");
    }};
}
