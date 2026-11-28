//! Scoperta minima dell'hardware, senza I/O: device tree (FDT) per ARM64 e RSDP/ACPI per x86-64.
//! Il kernel passa la memoria letta; qui si cerca solo ciò che serve.
#![cfg_attr(not(test), no_std)]

const FDT_MAGIC: u32 = 0xD00D_FEED;
const TOK_BEGIN: u32 = 1;
const TOK_END: u32 = 2;
const TOK_PROP: u32 = 3;
const TOK_NOP: u32 = 4;
const TOK_STOP: u32 = 9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Truncated,
    BadMagic,
    BadStructure,
}

fn be32(b: &[u8], off: usize) -> Result<u32, Error> {
    let s = b.get(off..off + 4).ok_or(Error::Truncated)?;
    Ok(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

fn align4(x: usize) -> usize {
    (x + 3) & !3
}

/// Cerca un nodo che abbia `compatible` con la stringa `name` (lista separata da NUL).
/// `Ok(true)` se almeno un nodo corrisponde.
pub fn has_compatible(blob: &[u8], name: &str) -> Result<bool, Error> {
    if be32(blob, 0)? != FDT_MAGIC {
        return Err(Error::BadMagic);
    }
    let total = be32(blob, 4)? as usize;
    let off_struct = be32(blob, 8)? as usize;
    let off_strings = be32(blob, 12)? as usize;
    let blob = blob.get(..total).ok_or(Error::Truncated)?;
    let mut p = off_struct;
    loop {
        match be32(blob, p)? {
            TOK_BEGIN => {
                // Nome del nodo: stringa NUL, allineata a 4.
                let start = p + 4;
                let end = blob.get(start..).ok_or(Error::Truncated)?
                    .iter().position(|&c| c == 0).ok_or(Error::BadStructure)?;
                p = align4(start + end + 1);
            }
            TOK_END | TOK_NOP => p += 4,
            TOK_PROP => {
                let len = be32(blob, p + 4)? as usize;
                let nameoff = be32(blob, p + 8)? as usize;
                let data = blob.get(p + 12..p + 12 + len).ok_or(Error::Truncated)?;
                let pname = blob.get(off_strings + nameoff..).ok_or(Error::Truncated)?;
                if cstr(pname) == b"compatible" && list_has(data, name.as_bytes()) {
                    return Ok(true);
                }
                p = align4(p + 12 + len);
            }
            TOK_STOP => return Ok(false),
            _ => return Err(Error::BadStructure),
        }
    }
}

fn cstr(b: &[u8]) -> &[u8] {
    let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    &b[..n]
}

fn list_has(list: &[u8], want: &[u8]) -> bool {
    list.split(|&c| c == 0).any(|s| s == want)
}

/// Cerca la RSDP ACPI ("RSD PTR ") su passi di 16 byte con checksum valido.
/// Restituisce l'offset nel buffer.
pub fn find_rsdp(mem: &[u8]) -> Option<usize> {
    const SIG: &[u8; 8] = b"RSD PTR ";
    let mut off = 0;
    while off + 20 <= mem.len() {
        if &mem[off..off + 8] == SIG && checksum_ok(&mem[off..off + 20]) {
            return Some(off);
        }
        off += 16;
    }
    None
}

/// Somma dei byte di una struttura ACPI: deve dare 0 modulo 256.
pub fn checksum_ok(b: &[u8]) -> bool {
    b.iter().fold(0u8, |a, &x| a.wrapping_add(x)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(v: &mut Vec<u8>, x: u32) {
        v.extend_from_slice(&x.to_be_bytes());
    }

    /// FDT minimo: radice con compatible "haiku,gba" e un nodo figlio "cpu" con "arm,cortex-a53".
    fn fdt() -> Vec<u8> {
        let strings = b"compatible\0".to_vec();
        let mut s = Vec::new();
        put(&mut s, TOK_BEGIN);
        s.extend_from_slice(b"\0\0\0\0"); // nome radice vuoto
        let list = b"haiku,gba\0";
        put(&mut s, TOK_PROP);
        put(&mut s, list.len() as u32);
        put(&mut s, 0);
        s.extend_from_slice(list);
        s.resize(align4(s.len()), 0);
        put(&mut s, TOK_BEGIN);
        s.extend_from_slice(b"cpu\0");
        let cpu = b"arm,cortex-a53\0";
        put(&mut s, TOK_PROP);
        put(&mut s, cpu.len() as u32);
        put(&mut s, 0);
        s.extend_from_slice(cpu);
        s.resize(align4(s.len()), 0);
        put(&mut s, TOK_END);
        put(&mut s, TOK_END);
        put(&mut s, TOK_STOP);

        let off_struct = 40u32;
        let off_strings = off_struct + s.len() as u32;
        let total = off_strings + strings.len() as u32;
        let mut h = Vec::new();
        put(&mut h, FDT_MAGIC);
        put(&mut h, total);
        put(&mut h, off_struct);
        put(&mut h, off_strings);
        while h.len() < 40 {
            h.push(0);
        }
        h.extend(s);
        h.extend(strings);
        h
    }

    #[test]
    fn trova_compatible_nel_nodo_figlio() {
        let b = fdt();
        assert_eq!(has_compatible(&b, "arm,cortex-a53"), Ok(true));
        assert_eq!(has_compatible(&b, "haiku,gba"), Ok(true));
        assert_eq!(has_compatible(&b, "altro,chip"), Ok(false));
    }

    #[test]
    fn rifiuta_magic_errato_e_troncato() {
        let mut b = fdt();
        b[0] ^= 1;
        assert_eq!(has_compatible(&b, "haiku,gba"), Err(Error::BadMagic));
        let b = fdt();
        assert_eq!(has_compatible(&b[..20], "haiku,gba"), Err(Error::Truncated));
    }

    #[test]
    fn rsdp_con_checksum() {
        let mut mem = vec![0u8; 64];
        let mut rsdp = b"RSD PTR ".to_vec();
        rsdp.resize(20, 0);
        let sum: u8 = rsdp.iter().fold(0u8, |a, &x| a.wrapping_add(x));
        rsdp[8] = 0u8.wrapping_sub(sum); // checksum: somma = 0
        mem[16..36].copy_from_slice(&rsdp);
        assert_eq!(find_rsdp(&mem), Some(16));
        mem[16 + 9] = 0xFF; // corrompe il checksum: nessuna RSDP
        assert_eq!(find_rsdp(&mem), None);
    }
}
