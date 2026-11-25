//! Microkernel alimentazione nel simulatore: macchina a stati con transizioni ammesse.
//! Il meccanismo è qui; la politica (quando dormire) resta al monolite. Nessun hardware reale.
//! Regole: docs/resource-microkernels.md.

use crate::resource::{Arch, Kind, Manifest};

/// Manifest generico: stati minimi, su tutte le architetture. Dettagli per macchina: da verificare.
pub const POWER_MANIFEST: Manifest = Manifest {
    name: "power-generic",
    kind: Kind::Power,
    archs: &[Arch::Gba, Arch::Arm64, Arch::X86_64],
    required: false,
    bytes: 1024,
    queue_depth: 4,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Active,
    Idle,
    Sleep,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerError {
    /// Transizione non ammessa dalla tabella.
    Illegal,
    /// Batteria sotto la soglia critica: non si torna ad Active.
    LowBattery,
}

pub struct Power {
    state: State,
    battery_pct: u8,
    critical_pct: u8,
}

impl Power {
    pub fn new(battery_pct: u8, critical_pct: u8) -> Self {
        Self { state: State::Off, battery_pct, critical_pct }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn set_battery(&mut self, pct: u8) {
        self.battery_pct = pct;
    }

    /// Transizioni: Off→Active (avvio); Active↔Idle; Idle→Sleep; Sleep→Active; qualsiasi→Off.
    pub fn request(&mut self, to: State) -> Result<(), PowerError> {
        use State::*;
        let ok = matches!(
            (self.state, to),
            (Off, Active)
                | (Active, Idle)
                | (Idle, Active)
                | (Idle, Sleep)
                | (Sleep, Active)
                | (Active, Off)
                | (Idle, Off)
                | (Sleep, Off)
        );
        if !ok {
            return Err(PowerError::Illegal);
        }
        if to == Active && self.state != Off && self.battery_pct < self.critical_pct {
            return Err(PowerError::LowBattery);
        }
        self.state = to;
        Ok(())
    }
}
