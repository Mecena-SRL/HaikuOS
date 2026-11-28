//! Parser del report descriptor USB HID (docs/kernel-study.md, riga 10). Puro: nessun I/O.
//! Produce la mappa dei campi di input: in quale bit sta ogni dato e quale usage ha.
//! Supporta campi variabili (assi, modificatori) e array (tasti). Voci lunghe si saltano.
//! Push/Pop non sono supportati: rispondono `Unsupported`.

pub const MAX_FIELDS: usize = 64;
const MAX_REPORTS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Truncated,
    Unsupported,
    TooManyFields,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Field {
    pub report_id: u8,
    pub page: u16,
    /// Usage del campo variabile; 0 per gli array (il valore è esso stesso un usage).
    pub usage: u16,
    pub bits: u8,
    /// Bit meno significativo dentro i dati del report (senza ID).
    pub offset: u32,
    pub logical_min: i32,
    pub logical_max: i32,
    pub array: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Layout {
    fields: [Field; MAX_FIELDS],
    len: usize,
}

impl Layout {
    pub fn fields(&self) -> &[Field] {
        &self.fields[..self.len]
    }
}

/// Legge un campo dai dati del report. Con minimo logico negativo il valore è con segno.
pub fn extract(report: &[u8], f: &Field) -> i32 {
    let mut v: u32 = 0;
    for k in 0..f.bits as u32 {
        let bit = f.offset + k;
        let byte = report.get((bit / 8) as usize).copied().unwrap_or(0);
        v |= (((byte >> (bit % 8)) & 1) as u32) << k;
    }
    let n = f.bits as u32;
    if f.logical_min < 0 && n > 0 && n < 32 && v & (1 << (n - 1)) != 0 {
        (v | (!0u32 << n)) as i32
    } else {
        v as i32
    }
}

fn usage_for(k: usize, usages: &[u16], nu: usize, umin: Option<u16>) -> u16 {
    if k < nu {
        usages[k]
    } else if let Some(m) = umin {
        m.wrapping_add(k as u16)
    } else if nu > 0 {
        usages[nu - 1]
    } else {
        0
    }
}

/// Offset in bit corrente per ogni report ID (0 = nessun ID).
fn report_bits<'a>(map: &'a mut [(u8, u32); MAX_REPORTS], n: &mut usize, id: u8) -> Result<&'a mut u32, Error> {
    let idx = match map[..*n].iter().position(|e| e.0 == id) {
        Some(i) => i,
        None => {
            if *n == MAX_REPORTS {
                return Err(Error::TooManyFields);
            }
            map[*n] = (id, 0);
            *n += 1;
            *n - 1
        }
    };
    Ok(&mut map[idx].1)
}

pub fn parse(desc: &[u8]) -> Result<Layout, Error> {
    let mut out = Layout { fields: [Field::default(); MAX_FIELDS], len: 0 };
    let (mut page, mut lmin, mut lmax) = (0u16, 0i32, 0i32);
    let (mut size, mut count, mut rid) = (0u32, 0u32, 0u8);
    let mut bits = [(0u8, 0u32); MAX_REPORTS];
    let mut nrep = 0usize;
    // Locali: si azzerano dopo ogni voce main.
    let mut usages = [0u16; 16];
    let (mut nu, mut umin): (usize, Option<u16>) = (0, None);
    let mut i = 0;
    while i < desc.len() {
        let b = desc[i];
        if b == 0xFE {
            // Voce lunga: prefisso, dimensione, tag, dati.
            let n = *desc.get(i + 1).ok_or(Error::Truncated)? as usize;
            i += 3 + n;
            continue;
        }
        let sz = match b & 3 {
            3 => 4,
            s => s as usize,
        };
        let end = i + 1 + sz;
        if end > desc.len() {
            return Err(Error::Truncated);
        }
        let data = desc[i + 1..end]
            .iter()
            .enumerate()
            .fold(0u32, |acc, (k, &x)| acc | (x as u32) << (8 * k));
        let sval = match sz {
            1 => data as u8 as i8 as i32,
            2 => data as u16 as i16 as i32,
            _ => data as i32,
        };
        let typ = (b >> 2) & 3;
        let tag = b >> 4;
        i = end;
        match (typ, tag) {
            (1, 0) => page = data as u16,
            (1, 1) => lmin = sval,
            (1, 2) => lmax = sval,
            (1, 7) => size = data,
            (1, 8) => rid = data as u8,
            (1, 9) => count = data,
            (1, 10) | (1, 11) => return Err(Error::Unsupported), // Push, Pop
            (2, 0) => {
                if nu < usages.len() {
                    usages[nu] = data as u16;
                    nu += 1;
                }
            }
            (2, 1) => umin = Some(data as u16),
            (2, 2) => {} // usage max: con umin basta a definire l'intervallo
            (0, 8) => {
                // Input: flags con bit 0 = costante (padding), bit 1 = variabile.
                if size > 32 {
                    return Err(Error::Unsupported);
                }
                let r = report_bits(&mut bits, &mut nrep, rid)?;
                if data & 1 != 0 {
                    *r += size * count;
                } else {
                    let var = data & 2 != 0;
                    for k in 0..count as usize {
                        if out.len == MAX_FIELDS {
                            return Err(Error::TooManyFields);
                        }
                        out.fields[out.len] = Field {
                            report_id: rid,
                            page,
                            usage: if var { usage_for(k, &usages, nu, umin) } else { 0 },
                            bits: size as u8,
                            offset: *r,
                            logical_min: lmin,
                            logical_max: lmax,
                            array: !var,
                        };
                        out.len += 1;
                        *r += size;
                    }
                }
                nu = 0;
                umin = None;
            }
            // Collection (0xA0), End Collection (0xC0), Output, Feature: non entrano nell'input.
            (0, _) => {
                nu = 0;
                umin = None;
            }
            _ => {}
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tastiera boot (descrittore di esempio dello standard HID, appendice B).
    const BOOT_KBD: &[u8] = &[
        0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x00, 0x25, 0x01,
        0x75, 0x01, 0x95, 0x08, 0x81, 0x02, 0x95, 0x01, 0x75, 0x08, 0x81, 0x01, 0x95, 0x05, 0x75, 0x01,
        0x05, 0x08, 0x19, 0x01, 0x29, 0x05, 0x91, 0x02, 0x95, 0x01, 0x75, 0x03, 0x91, 0x01, 0x95, 0x06,
        0x75, 0x08, 0x15, 0x00, 0x25, 0x65, 0x05, 0x07, 0x19, 0x00, 0x29, 0x65, 0x81, 0x00, 0xC0,
    ];

    #[test]
    fn tastiera_boot_modificatori_e_tasti() {
        let l = parse(BOOT_KBD).unwrap();
        let f = l.fields();
        assert_eq!(f.len(), 14); // 8 modificatori + 6 tasti
        assert_eq!((f[0].usage, f[0].offset, f[0].bits), (0xE0, 0, 1));
        assert_eq!((f[7].usage, f[7].offset), (0xE7, 7));
        assert!(f[8].array && f[8].offset == 16 && f[8].bits == 8);
        let rep = [0x02, 0x00, 0x04, 0, 0, 0, 0, 0];
        assert_eq!(extract(&rep, &f[1]), 1); // LeftShift premuto
        assert_eq!(extract(&rep, &f[8]), 0x04); // primo tasto: 'a'
        assert_eq!(extract(&rep, &f[9]), 0);
    }

    #[test]
    fn asse_con_segno() {
        // X: minimo -127, massimo 127, 8 bit, variabile.
        let d = [0x05, 0x01, 0x09, 0x30, 0x15, 0x81, 0x25, 0x7F, 0x75, 0x08, 0x95, 0x01, 0x81, 0x02, 0xC0];
        let l = parse(&d).unwrap();
        assert_eq!(extract(&[0xFE], &l.fields()[0]), -2);
    }

    #[test]
    fn descrittore_troncato() {
        assert_eq!(parse(&[0x05]), Err(Error::Truncated));
    }
}
