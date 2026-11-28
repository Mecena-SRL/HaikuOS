//! Mappa memoria BIOS (E820) lasciata da stage2 a 0x0A00: conteggio, poi voci da 24 byte.
const COUNT: usize = 0x0A00;
const ENTRIES: usize = 0x0A10;

/// Byte di RAM utilizzabile (tipo 1, voci della mappa BIOS).
pub fn usable() -> u64 {
    let mut usable = 0u64;
    unsafe {
        let n = core::ptr::read_volatile(COUNT as *const u32) as usize;
        for i in 0..n.min(16) {
            let e = ENTRIES + i * 24;
            let len = core::ptr::read_volatile((e + 8) as *const u64);
            let kind = core::ptr::read_volatile((e + 16) as *const u32);
            if kind == 1 {
                usable += len;
            }
        }
    }
    usable
}

/// Allocatore di pagine fisiche: 64K pagine (256 MB), blocchi fino a 2^16 pagine.
type Phys = haiku_memcore::Buddy<1024, 17>;
static mut PHYS: Phys = Phys::new();

/// Riempie l'allocatore con le regioni utili della mappa BIOS (pagine da 4 KB).
/// Solo la prima parte di RAM sotto 256 MB, il resto viene ignorato.
pub fn init_phys() -> usize {
    let mut pages = 0usize;
    unsafe {
        let n = core::ptr::read_volatile(COUNT as *const u32) as usize;
        let phys = &mut *core::ptr::addr_of_mut!(PHYS);
        for i in 0..n.min(16) {
            let e = ENTRIES + i * 24;
            let base = core::ptr::read_volatile(e as *const u64);
            let len = core::ptr::read_volatile((e + 8) as *const u64);
            let kind = core::ptr::read_volatile((e + 16) as *const u32);
            if kind != 1 {
                continue;
            }
            let first = (base as usize).div_ceil(4096);
            let end = ((base + len) as usize / 4096).min(Phys::PAGES);
            if end > first {
                phys.release(first, end - first);
                pages += end - first;
            }
        }
    }
    pages
}

/// Alloca 2^order pagine fisiche; restituisce l'indirizzo fisico della prima.
pub fn alloc_pages(order: usize) -> Option<u64> {
    unsafe {
        let phys = &mut *core::ptr::addr_of_mut!(PHYS);
        phys.alloc(order).map(|p| (p as u64) << 12)
    }
}

/// Libera 2^order pagine partendo dall'indirizzo fisico `addr` (allineato a 4 KB).
pub fn free_pages(addr: u64, order: usize) {
    unsafe {
        let phys = &mut *core::ptr::addr_of_mut!(PHYS);
        phys.free((addr >> 12) as usize, order);
    }
}
