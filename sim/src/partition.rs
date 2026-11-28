//! Lettura di sola lettura delle tabelle delle partizioni: MBR e GPT (UEFI 2.x, sez. 5).
//! Lavora su un'immagine di disco in memoria. Nessun file esterno, nessuna scrittura.
//! Specifica: UEFI 2.10, sez. 5.3 (GPT) e 5.2.1 (MBR). **Da verificare** sul testo ufficiale.
//! Formattazione, partizionamento e cifratura sono programmi, non nucleo: questo modulo
//! sta nel programma di gestione dischi, non nel kernel.

pub const SECTOR: usize = 512;
/// Limite del prototipo: GPT standard, 128 voci da 128 byte. **Da verificare**: la specifica ammette altro.
pub const GPT_MAX_ENTRIES: usize = 128;
pub const GPT_ENTRY_BYTES: usize = 128;
/// Tipo MBR per la MBR protettiva di un disco GPT.
pub const MBR_KIND_GPT_PROTECTIVE: u8 = 0xEE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartError {
    /// Disco troppo piccolo per la struttura letta.
    TooSmall,
    /// Firma assente (0x55AA per MBR, "EFI PART" per GPT).
    NoSignature,
    /// CRC32 non corrispondente: intestazione o array di voci alterati.
    BadCrc,
    /// Campo fuori dai valori ammessi dal prototipo.
    BadHeader,
    /// Voce che punta fuori dal disco o fuori dall'area utilizzabile.
    OutOfRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MbrEntry {
    pub kind: u8,
    pub first_lba: u32,
    pub sectors: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GptPartition {
    /// Tipo in forma testuale, es. "EBD0A0A2-B9E5-4433-87C0-68B6B72699C7".
    pub type_guid: String,
    pub unique_guid: String,
    pub first_lba: u64,
    pub last_lba: u64,
    pub name: String,
}

/// CRC32 IEEE (poli riflesso 0xEDB88320), come usato da GPT. Bit a bit: piccolo e lento, va bene qui.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

fn u16_at(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}
fn u64_at(d: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(d[o..o + 8].try_into().unwrap())
}

/// GUID su disco: primi tre campi little-endian, resto big-endian. Restituisce la forma testuale maiuscola.
pub fn guid_to_string(b: &[u8]) -> String {
    format!(
        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        u32_at(b, 0),
        u16_at(b, 4),
        u16_at(b, 6),
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

/// Voci MBR non vuote (tipo diverso da 0). Richiede la firma 0x55AA a fine settore.
pub fn read_mbr(disk: &[u8]) -> Result<Vec<MbrEntry>, PartError> {
    if disk.len() < SECTOR {
        return Err(PartError::TooSmall);
    }
    if u16_at(disk, 510) != 0xAA55 {
        return Err(PartError::NoSignature);
    }
    let mut out = Vec::new();
    for i in 0..4 {
        let o = 446 + i * 16;
        let kind = disk[o + 4];
        if kind != 0 {
            out.push(MbrEntry { kind, first_lba: u32_at(disk, o + 8), sectors: u32_at(disk, o + 12) });
        }
    }
    Ok(out)
}

/// Vero se la MBR contiene la voce protettiva di un disco GPT.
pub fn is_gpt_protective(mbr: &[MbrEntry]) -> bool {
    mbr.iter().any(|e| e.kind == MBR_KIND_GPT_PROTECTIVE)
}

/// Legge la intestazione GPT al settore 1 e l'array di voci. Verifica entrambi i CRC32,
/// la posizione (LBA 1), i limiti del disco e quelli dell'area utilizzabile.
/// Non legge l'intestazione di backup: **da verificare** (serve per il ripristino).
pub fn read_gpt(disk: &[u8]) -> Result<Vec<GptPartition>, PartError> {
    if disk.len() < 2 * SECTOR {
        return Err(PartError::TooSmall);
    }
    let h = &disk[SECTOR..2 * SECTOR];
    if &h[0..8] != b"EFI PART" {
        return Err(PartError::NoSignature);
    }
    let header_size = u32_at(h, 12) as usize;
    if !(92..=SECTOR).contains(&header_size) {
        return Err(PartError::BadHeader);
    }
    // CRC dell'intestazione: calcolato con il campo CRC (offset 16) azzerato.
    let mut hz = h[..header_size].to_vec();
    hz[16..20].copy_from_slice(&[0; 4]);
    if crc32(&hz) != u32_at(h, 16) {
        return Err(PartError::BadCrc);
    }
    if u64_at(h, 24) != 1 {
        return Err(PartError::BadHeader);
    }
    let first_usable = u64_at(h, 40);
    let last_usable = u64_at(h, 48);
    let entries_lba = u64_at(h, 72);
    let entry_count = u32_at(h, 80) as usize;
    let entry_size = u32_at(h, 84) as usize;
    let entries_crc = u32_at(h, 88);
    if entry_size != GPT_ENTRY_BYTES || entry_count == 0 || entry_count > GPT_MAX_ENTRIES {
        return Err(PartError::BadHeader);
    }
    if first_usable > last_usable {
        return Err(PartError::BadHeader);
    }
    let start = (entries_lba as usize).checked_mul(SECTOR).ok_or(PartError::OutOfRange)?;
    let len = entry_count * entry_size;
    let end = start.checked_add(len).ok_or(PartError::OutOfRange)?;
    if end > disk.len() {
        return Err(PartError::OutOfRange);
    }
    let arr = &disk[start..end];
    if crc32(arr) != entries_crc {
        return Err(PartError::BadCrc);
    }

    let mut out = Vec::new();
    for e in arr.chunks_exact(GPT_ENTRY_BYTES) {
        if e[0..16].iter().all(|&b| b == 0) {
            continue; // voce libera
        }
        let first = u64_at(e, 32);
        let last = u64_at(e, 40);
        if first > last || first < first_usable || last > last_usable {
            return Err(PartError::OutOfRange);
        }
        if (last as usize + 1).checked_mul(SECTOR).map_or(true, |x| x > disk.len()) {
            return Err(PartError::OutOfRange);
        }
        let units: Vec<u16> = e[56..128]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&u| u != 0)
            .collect();
        out.push(GptPartition {
            type_guid: guid_to_string(&e[0..16]),
            unique_guid: guid_to_string(&e[16..32]),
            first_lba: first,
            last_lba: last,
            name: String::from_utf16_lossy(&units),
        });
    }
    Ok(out)
}
