// Enumerazione PCI (bus 0) via porte di configurazione legacy 0xCF8/0xCFC.
// Serve a rilevare le schede prima di caricare i driver: nessuna lista di modelli.
use crate::console;
use crate::io::{inl, outl};

const ADDR: u16 = 0xCF8;
const DATA: u16 = 0xCFC;

fn read(bus: u8, dev: u8, func: u8, reg: u8) -> u32 {
    let addr = 0x8000_0000u32
        | (bus as u32) << 16
        | (dev as u32 & 0x1F) << 11
        | (func as u32 & 0x7) << 8
        | (reg as u32 & 0xFC);
    unsafe {
        outl(ADDR, addr);
        inl(DATA)
    }
}

/// Un dispositivo trovato: classe e sottoclasse PCI (es. 0x01/0x06 = AHCI).
#[derive(Clone, Copy)]
pub struct Dev {
    pub vendor: u16,
    pub class: u8,
    pub sub: u8,
    pub prog_if: u8,
}

/// Scansiona il bus 0 e chiama `f` per ogni funzione presente.
pub fn scan(mut f: impl FnMut(Dev)) {
    for dev in 0..32u8 {
        let id = read(0, dev, 0, 0);
        if id & 0xFFFF == 0xFFFF {
            continue;
        }
        let multi = (read(0, dev, 0, 0x0C) >> 16) & 0x80 != 0;
        let funcs = if multi { 8 } else { 1 };
        for func in 0..funcs {
            let id = read(0, dev, func, 0);
            if id & 0xFFFF == 0xFFFF {
                continue;
            }
            let cls = read(0, dev, func, 0x08);
            f(Dev {
                vendor: (id & 0xFFFF) as u16,
                class: (cls >> 24) as u8,
                sub: (cls >> 16) as u8,
                prog_if: (cls >> 8) as u8,
            });
        }
    }
}

/// Nome leggibile della classe, solo per il log di avvio.
pub fn class_name(d: Dev) -> &'static str {
    match (d.class, d.sub, d.prog_if) {
        (0x01, 0x06, _) => "SATA AHCI",
        (0x01, 0x08, 0x02) => "NVMe",
        (0x01, 0x01, _) => "IDE",
        (0x02, _, _) => "rete",
        (0x03, _, _) => "video",
        (0x04, 0x03, _) => "audio HDA",
        (0x06, 0x04, _) => "bridge PCI",
        (0x0C, 0x03, 0x30) => "USB xHCI",
        _ => "altro",
    }
}

pub fn report() -> u32 {
    let mut n = 0;
    scan(|d| {
        n += 1;
        console::puts("[pci] ");
        console::puts(class_name(d));
        console::puts("\n");
    });
    n
}
