//! Driver del dispositivo di esempio: registri STATUS, DATA, CMD.
use crate::bus::Bus;

pub const REG_STATUS: u32 = 0x0;
pub const REG_DATA: u32 = 0x4;
pub const REG_CMD: u32 = 0x8;

/// bit0: c'e' un byte pronto in DATA.
pub const ST_DATA_READY: u32 = 1 << 0;
/// bit1: il dispositivo sta elaborando un comando.
pub const ST_BUSY: u32 = 1 << 1;

/// Numero massimo di poll dello stato prima di arrendersi.
pub const MAX_POLLS: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Busy,
}

pub struct Driver<B: Bus> {
    bus: B,
}

impl<B: Bus> Driver<B> {
    pub fn new(bus: B) -> Self {
        Driver { bus }
    }

    pub fn bus(&self) -> &B {
        &self.bus
    }

    /// Legge un byte (e lo toglie dalla FIFO del dispositivo), o None se non c'e' dato.
    pub fn read_byte(&mut self) -> Option<u8> {
        if self.bus.read_reg(REG_STATUS) & ST_DATA_READY == 0 {
            return None;
        }
        Some(self.bus.read_reg(REG_DATA) as u8)
    }

    /// Invia un comando appena il dispositivo non e' piu' occupato; Err(Busy) dopo MAX_POLLS.
    pub fn send_cmd(&mut self, cmd: u8) -> Result<(), Error> {
        for _ in 0..MAX_POLLS {
            if self.bus.read_reg(REG_STATUS) & ST_BUSY == 0 {
                self.bus.write_reg(REG_CMD, cmd as u32);
                return Ok(());
            }
            self.bus.wait();
        }
        Err(Error::Busy)
    }
}
