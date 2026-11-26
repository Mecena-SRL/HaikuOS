// Driver ATA PIO (LBA28) sul canale primario, disco master.
use crate::io::{inb, inw, outb};

const DATA: u16 = 0x1F0;
const STATUS: u16 = 0x1F7;

/// Legge un settore da 512 byte. Restituisce false se il disco non risponde o dà errore.
pub fn read_sector(lba: u32, buf: &mut [u8; 512]) -> bool {
    unsafe {
        if !wait(false) {
            return false;
        }
        outb(0x1F6, 0xE0 | ((lba >> 24) as u8 & 0x0F));
        outb(0x1F2, 1);
        outb(0x1F3, lba as u8);
        outb(0x1F4, (lba >> 8) as u8);
        outb(0x1F5, (lba >> 16) as u8);
        outb(STATUS, 0x20); // comando: READ SECTORS
        if !wait(true) {
            return false;
        }
        for i in 0..256 {
            let w = inw(DATA);
            buf[2 * i] = w as u8;
            buf[2 * i + 1] = (w >> 8) as u8;
        }
        true
    }
}

/// Attende che il disco non sia occupato; con `drq` attende anche i dati pronti.
unsafe fn wait(drq: bool) -> bool {
    for _ in 0..100_000 {
        let s = inb(STATUS);
        if s == 0xFF {
            return false; // nessun disco sul bus (UTM/AHCI): niente attesa
        }
        if s & 0x80 == 0 {
            if s & 0x21 != 0 {
                return false; // ERR o DF
            }
            if !drq || s & 0x08 != 0 {
                return true;
            }
        }
    }
    false
}
