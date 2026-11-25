//! Quattro token (bibbia §3). Ogni elemento usa solo il token del suo ruolo.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tok {
    T0,
    T1,
    T2,
    T3,
}

impl Tok {
    /// Formato GBA 15 bit: r | g<<5 | b<<10.
    pub const fn gba(self) -> u16 {
        match self {
            Tok::T0 => 1 | (2 << 5) | (6 << 10),
            Tok::T1 => 31 | (31 << 5) | (31 << 10),
            Tok::T2 => 31 | (16 << 5),
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
        }
    }
}
