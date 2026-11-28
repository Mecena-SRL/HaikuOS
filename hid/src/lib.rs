//! Parser del report descriptor USB HID (docs/kernel-study.md, riga 10). Nessun I/O, nessuna allocazione.
//! Produce l'elenco dei campi: in quale bit del report sta ogni dato e a quale usage corrisponde.
#![cfg_attr(not(test), no_std)]

/// Un campo del report: `size` bit, `count` volte, a partire dal bit `bit` (dall'inizio del report).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Field {
    pub report_id: u8,
    pub bit: u32,
    pub size: u32,
    pub count: u32,
    pub usage_page: u16,
    /// Primo usage dei campi (0 se assente).
    pub usage: u16,
    pub logical_min: i32,
    pub logical_max: i32,
    /// Input (true) o output (false).
    pub input: bool,
    /// Costante/padding (true) o dato (false).
    pub constant: bool,
    /// Variabile (true): un valore per usage. Array (false): indice di usage.
    pub variable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Descrittore troncato o item con lunghezza non valida.
    Truncated,
    /// Lo spazio `out` è pieno.
    Full,
    /// Pop senza push, o End Collection senza Collection.
    Stack,
}

#[derive(Clone, Copy, Default)]
struct State {
    usage_page: u16,
    logical_min: i32,
    logical_max: i32,
    size: u32,
    count: u32,
    report_id: u8,
}

/// Analizza il descrittore e scrive i campi di input/output/feature in `out`.
/// Ritorna quanti campi sono stati scritti.
pub fn parse(desc: &[u8], out: &mut [Field]) -> Result<usize, Error> {
    let mut st = State::default();
    let mut stack = [State::default(); 4];
    let mut sp = 0usize;
    let mut usages = [0u16; 8];
    let mut nusages = 0usize;
    // Bit correnti per ogni report id (fino a 4 report distinti).
    let mut bits = [(0u8, 0u32); 4];
    let mut nbits = 0usize;
    let mut n = 0usize;
    let mut i = 0usize;

    while i < desc.len() {
        let prefix = desc[i];
        if prefix == 0xFE {
            return Err(Error::Truncated); // item lungo: non supportato, non serve per le periferiche comuni
        }
        let len = match prefix & 3 {
            3 => 4,
            n => n as usize,
        };
        if i + 1 + len > desc.len() {
            return Err(Error::Truncated);
        }
        let data = &desc[i + 1..i + 1 + len];
        let val = data.iter().rev().fold(0u32, |a, &b| (a << 8) | b as u32);
        let sval = match len {
            1 => data[0] as i8 as i32,
            2 => i16::from_le_bytes([data[0], data[1]]) as i32,
            4 => val as i32,
            _ => 0,
        };
        let tag = prefix & 0xFC;
        match tag {
            // Global
            0x04 => st.usage_page = val as u16,
            0x14 => st.logical_min = sval,
            0x24 => st.logical_max = sval,
            0x74 => st.size = val,
            0x94 => st.count = val,
            0x84 => st.report_id = val as u8,
            0xA4 => {
                if sp == stack.len() {
                    return Err(Error::Stack);
                }
                stack[sp] = st;
                sp += 1;
            }
            0xB4 => {
                if sp == 0 {
                    return Err(Error::Stack);
                }
                sp -= 1;
                let rid = st.report_id;
                st = stack[sp];
                st.report_id = rid;
            }
            // Local
            0x08 => {
                if nusages < usages.len() {
                    usages[nusages] = val as u16;
                    nusages += 1;
                }
            }
            0x18 | 0x28 => {} // usage min/max: non espansi, il primo usage basta per ora
            // Main
            0x80 | 0x90 | 0xB0 => {
                // Input, Output, Feature.
                let slot = match bits[..nbits].iter().position(|b| b.0 == st.report_id) {
                    Some(p) => p,
                    None => {
                        if nbits == bits.len() {
                            return Err(Error::Full);
                        }
                        bits[nbits] = (st.report_id, 0);
                        nbits += 1;
                        nbits - 1
                    }
                };
                if n == out.len() {
                    return Err(Error::Full);
                }
                let bit = bits[slot].1;
                out[n] = Field {
                    report_id: st.report_id,
                    bit,
                    size: st.size,
                    count: st.count,
                    usage_page: st.usage_page,
                    usage: if nusages > 0 { usages[0] } else { 0 },
                    logical_min: st.logical_min,
                    logical_max: st.logical_max,
                    input: tag == 0x80,
                    constant: val & 1 != 0,
                    variable: val & 2 != 0,
                };
                n += 1;
                bits[slot].1 += st.size * st.count;
                nusages = 0;
            }
            0xA0 => nusages = 0, // Collection: le usage locali si consumano
            0xC0 => nusages = 0, // End Collection
            _ => {}              // tag sconosciuti: ignorati
        }
        i += 1 + len;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tastiera boot (HID 1.11, appendice B): 8 bit di modificatori, 1 riservato,
    // 5 LED di output, 3 bit di padding, 6 codici tasto da 1 byte.
    const KBD: [u8; 63] = [
        0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x00, 0x25,
        0x01, 0x75, 0x01, 0x95, 0x08, 0x81, 0x02, 0x95, 0x01, 0x75, 0x08, 0x81, 0x01, 0x95, 0x05,
        0x75, 0x01, 0x05, 0x08, 0x19, 0x01, 0x29, 0x05, 0x91, 0x02, 0x95, 0x01, 0x75, 0x03, 0x91,
        0x01, 0x05, 0x07, 0x19, 0x00, 0x29, 0x65, 0x15, 0x00, 0x25, 0x65, 0x75, 0x08, 0x95, 0x06,
        0x81, 0x00, 0xC0,
    ];

    #[test]
    fn tastiera_boot() {
        let mut out = [Field::default(); 8];
        let n = parse(&KBD, &mut out).unwrap();
        assert_eq!(n, 5);
        // Modificatori: 8 x 1 bit, variabili, input.
        assert_eq!((out[0].usage_page, out[0].size, out[0].count), (0x07, 1, 8));
        assert_eq!((out[0].bit, out[0].logical_max, out[0].variable), (0, 1, true));
        // Riservato: costante, 8 bit, a partire dal bit 8.
        assert!(out[1].constant && out[1].bit == 8);
        // LED: output, 5 bit, usage page 0x08.
        assert!(!out[2].input && out[2].usage_page == 0x08 && out[2].count == 5);
        assert_eq!(out[2].bit, 16);
        // Padding dopo i LED: 3 bit, dal bit 21.
        assert!(out[3].constant && out[3].bit == 21 && out[3].size == 3);
        // Codici tasto: array di 6 byte, dal bit 24.
        assert!(!out[4].variable && out[4].bit == 24 && out[4].count == 6);
        assert_eq!((out[4].logical_min, out[4].logical_max), (0, 0x65));
    }

    #[test]
    fn troncato_e_pieno() {
        let mut out = [Field::default(); 8];
        assert_eq!(parse(&KBD[..9], &mut out), Err(Error::Truncated));
        let mut one = [Field::default(); 1];
        assert_eq!(parse(&KBD, &mut one), Err(Error::Full));
    }

    #[test]
    fn stack_sbilanciato() {
        let mut out = [Field::default(); 2];
        assert_eq!(parse(&[0xB4], &mut out), Err(Error::Stack));
    }

    #[test]
    fn report_id_separati() {
        // Due report con ID 1 e 2: ognuno parte dal bit 0.
        let d = [
            0x05, 0x01, 0x85, 0x01, 0x75, 0x08, 0x95, 0x01, 0x81, 0x01, 0x85, 0x02, 0x75, 0x04,
            0x95, 0x02, 0x81, 0x01,
        ];
        let mut out = [Field::default(); 4];
        let n = parse(&d, &mut out).unwrap();
        assert_eq!(n, 2);
        assert_eq!((out[0].report_id, out[0].bit), (1, 0));
        assert_eq!((out[1].report_id, out[1].bit, out[1].size), (2, 0, 4));
    }
}
