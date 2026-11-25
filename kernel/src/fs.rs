// Lettura del file system minimale HAIKUFS1 (superblocco a LBA 2048).
// Formato: magic[8], u32 numero file, entry da 32 byte a partire da 16:
//   nome[16], u32 lba iniziale, u32 dimensione in byte, 8 byte riservati.
use crate::ata::read_sector;
use crate::console;

const SUPER_LBA: u32 = 2048;

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn name_of(b: &[u8]) -> &str {
    let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    core::str::from_utf8(&b[..n]).unwrap_or("?")
}

/// Elenca i file della radice e stampa il contenuto del primo. Restituisce false se non c'è il magic.
pub fn show_root() -> bool {
    let mut sb = [0u8; 512];
    if !read_sector(SUPER_LBA, &mut sb) || &sb[0..8] != b"HAIKUFS1" {
        return false;
    }
    let count = (u32_at(&sb, 8) as usize).min(14);
    kprintln!("FS HAIKUFS1: file trovati:");
    console::put_dec(count as u32);
    kprintln!("");
    for i in 0..count {
        let e = 16 + i * 32;
        kprintln!("  file:");
        kprintln!(name_of(&sb[e..e + 16]));
        console::put_dec(u32_at(&sb, e + 20));
        kprintln!(" byte");
    }
    if count > 0 {
        let lba = u32_at(&sb, 32);
        let size = (u32_at(&sb, 36) as usize).min(512);
        let mut data = [0u8; 512];
        if read_sector(lba, &mut data) {
            match core::str::from_utf8(&data[..size]) {
                Ok(t) => kprintln!(t),
                Err(_) => kprintln!("(contenuto non testuale)"),
            }
        }
    }
    true
}
