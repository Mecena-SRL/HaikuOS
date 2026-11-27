// Controller SATA AHCI: rileva le porte con un disco collegato. Solo lettura dei registri:
// nessun comando al disco in questo passo.
use crate::console;
use crate::pci::{self, Dev};

const HBA_GHC_AE: u32 = 1 << 31;

unsafe fn rd(base: usize, off: usize) -> u32 {
    core::ptr::read_volatile((base + off) as *const u32)
}

/// Cerca il controller AHCI (classe 01h, sottoclasse 06h, interfaccia 01h) e stampa le porte.
/// Ritorna il numero di dischi SATA trovati.
pub fn probe() -> u32 {
    let mut found: Option<Dev> = None;
    pci::scan(|d| {
        if d.class == 0x01 && d.sub == 0x06 && d.prog_if == 0x01 && found.is_none() {
            found = Some(d);
        }
    });
    let Some(d) = found else {
        console::puts("[ahci] nessun controller AHCI\n");
        return 0;
    };
    // BAR5 (ABAR): registri del controller, in memoria.
    let abar = (pci::read(0, d.dev, d.func, 0x24) & 0xFFFF_FFF0) as usize;
    // Abilita memoria (bit 1) e bus master (bit 2) nel registro di comando.
    let cmd = pci::read(0, d.dev, d.func, 0x04);
    pci::write(d.dev, d.func, 0x04, cmd | 0b110);
    if abar == 0 {
        console::puts("[ahci] BAR5 non valido\n");
        return 0;
    }
    let mut dischi = 0;
    unsafe {
        // GHC.AE: abilita la modalità AHCI.
        let ghc = rd(abar, 0x04);
        core::ptr::write_volatile((abar + 0x04) as *mut u32, ghc | HBA_GHC_AE);
        let pi = rd(abar, 0x0C); // porte implementate
        for i in 0..32usize {
            if pi & (1 << i) == 0 {
                continue;
            }
            let p = abar + 0x100 + 0x80 * i;
            let ssts = rd(p, 0x28);
            let present = ssts & 0xF == 3 && (ssts >> 8) & 0xF == 1; // dispositivo e collegamento attivi
            if !present {
                continue;
            }
            let sig = rd(p, 0x24);
            let tipo = match sig {
                0x0000_0101 => "disco SATA",
                0xEB14_0101 => "unita' ATAPI",
                _ => "altro",
            };
            if sig == 0x0000_0101 {
                dischi += 1;
            }
            console::puts("[ahci] porta ");
            console::put_dec(i as u32);
            console::puts(": ");
            console::puts(tipo);
            console::puts("\n");
        }
    }
    dischi
}
