//! Quattro token (bibbia §3). Ogni elemento usa solo il token del suo ruolo.
//! x86 aggiunge `A(k)`: colori di app e di pulsante a 32 bit (bibbia §3.1). GBA: `A(_)` = T2.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tok {
    T0,
    T1,
    T2,
    T3,
    /// Accento per indice (`ACCENTS`): titolo di app 1..8, pulsanti 9 (riduci), 10 (massimizza), 11 (chiudi).
    A(u8),
}

/// Accenti x86 a 32 bit (0xRRGGBB). Indice = id app per i titoli, 9..11 per i pulsanti.
pub const ACCENTS: [u32; 12] = [
    0x00_00_00, // 0: non usato
    0xF2_C1_4E, // 1 FILE: giallo cartella
    0x4E_CD_C4, // 2 TERM: verde acqua
    0xFF_8C_42, // 3 CALC: arancio
    0x6E_A8_FE, // 4 WEB: blu
    0xB3_92_F0, // 5 EDIT: lilla
    0xA0_AE_C0, // 6 SET: grigio azzurro
    0xFF_6B_9D, // 7 ORA: rosa
    0x8B_D1_7C, // 8 READ: verde
    0xFF_D6_0A, // 9 riduci
    0x3D_DC_84, // 10 massimizza
    0xFF_5F_57, // 11 chiudi
];

impl Tok {
    /// Formato GBA 15 bit: r | g<<5 | b<<10. `A(_)` non esiste su GBA: vale T2.
    pub const fn gba(self) -> u16 {
        match self {
            Tok::T0 => 1 | (2 << 5) | (6 << 10),
            Tok::T1 => 31 | (31 << 5) | (31 << 10),
            Tok::T2 | Tok::A(_) => 31 | (16 << 5),
            Tok::T3 => (20 << 5) | (31 << 10),
        }
    }

    /// sRGB 0xRRGGBB per schermi grandi (x86 framebuffer, Mac).
    pub const fn rgb(self) -> u32 {
        match self {
            Tok::T0 => 0x08_10_31,
            Tok::T1 => 0xFF_FF_FF,
            Tok::T2 => 0xFF_84_00,
            Tok::T3 => 0x00_A5_FF,
            Tok::A(k) => ACCENTS[k as usize % ACCENTS.len()],
        }
    }
}
