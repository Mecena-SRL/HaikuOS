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

const MAX_FILES: usize = 15;
const ARENA: usize = 8192;
static mut ARENA_BUF: [u8; ARENA] = [0; ARENA];
static mut NAMES: [[u8; 16]; MAX_FILES] = [[0; 16]; MAX_FILES];
static mut LIST: [(&str, &str); MAX_FILES] = [("", ""); MAX_FILES];

/// Tutti i file della radice di HAIKUFS1 (nome, testo), letti dal disco.
/// Vuoto se il disco non c'è o non ha il magic. Testo oltre 8 KB viene troncato.
pub fn list() -> &'static [(&'static str, &'static str)] {
    unsafe {
        let arena = &mut *core::ptr::addr_of_mut!(ARENA_BUF);
        let names = &mut *core::ptr::addr_of_mut!(NAMES);
        let mut sb = [0u8; 512];
        if !read_sector(SUPER_LBA, &mut sb) || &sb[0..8] != b"HAIKUFS1" {
            return &[];
        }
        let count = (u32_at(&sb, 8) as usize).min(MAX_FILES);
        // Fase 1: lettura. Per ogni file: lunghezza del nome e del testo nell'arena.
        let mut meta = [(0usize, 0usize, 0usize); MAX_FILES];
        let mut used = 0;
        for i in 0..count {
            let e = 16 + i * 32;
            let nl = sb[e..e + 16].iter().position(|&c| c == 0).unwrap_or(16);
            names[i][..nl].copy_from_slice(&sb[e..e + nl]);
            let lba = u32_at(&sb, e + 16);
            let len = (u32_at(&sb, e + 20) as usize).min(ARENA - used);
            let mut off = 0;
            while off < len {
                let mut s = [0u8; 512];
                if !read_sector(lba + (off / 512) as u32, &mut s) {
                    break;
                }
                let k = (len - off).min(512);
                arena[used + off..used + off + k].copy_from_slice(&s[..k]);
                off += 512;
            }
            meta[i] = (nl, used, len);
            used += len;
        }
        // Fase 2: solo letture, poi le str puntano ai buffer statici.
        let names = &*core::ptr::addr_of!(NAMES);
        let arena = &*core::ptr::addr_of!(ARENA_BUF);
        let list = &mut *core::ptr::addr_of_mut!(LIST);
        for i in 0..count {
            let (nl, off, len) = meta[i];
            let name = core::str::from_utf8(&names[i][..nl]).unwrap_or("?");
            let body = core::str::from_utf8(&arena[off..off + len]).unwrap_or("(non testuale)");
            list[i] = (name, body);
        }
        &list[..count]
    }
}
