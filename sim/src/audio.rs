//! Microkernel audio nel simulatore: mixer a canali con buffer fissi.
//! Campioni i16, mix con saturazione, conteggio degli underrun. Nessun hardware reale.
//! Regole: docs/resource-microkernels.md.

use crate::resource::{Arch, Kind, Manifest};

/// Manifest generico: mixer a campioni, su tutte le architetture.
pub const AUDIO_MANIFEST: Manifest = Manifest {
    name: "audio-mixer-generic",
    kind: Kind::Audio,
    archs: &[Arch::Gba, Arch::Arm64, Arch::X86_64],
    required: false,
    bytes: 4 * 1024,
    queue_depth: 4,
};

pub const MAX_CHANNELS: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    /// Volume 0..=255. 255 = pieno.
    pub volume: u8,
    samples: std::collections::VecDeque<i16>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AudioError {
    NoChannel,
    NoSlot,
}

pub struct Mixer {
    channels: Vec<Channel>,
    underruns: u32,
}

impl Mixer {
    pub fn new(channels: usize) -> Result<Self, AudioError> {
        if channels == 0 || channels > MAX_CHANNELS {
            return Err(AudioError::NoSlot);
        }
        let ch = Channel { volume: 255, samples: Default::default() };
        Ok(Self { channels: vec![ch; channels], underruns: 0 })
    }

    pub fn push(&mut self, ch: usize, samples: &[i16]) -> Result<(), AudioError> {
        let c = self.channels.get_mut(ch).ok_or(AudioError::NoChannel)?;
        c.samples.extend(samples.iter().copied());
        Ok(())
    }

    pub fn set_volume(&mut self, ch: usize, volume: u8) -> Result<(), AudioError> {
        self.channels.get_mut(ch).ok_or(AudioError::NoChannel)?.volume = volume;
        Ok(())
    }

    /// Mixa `frames` campioni. Un canale senza dati aggiunge silenzio e conta un underrun.
    pub fn mix(&mut self, frames: usize) -> Vec<i16> {
        let mut out = vec![0i16; frames];
        for c in self.channels.iter_mut() {
            if c.samples.len() < frames {
                self.underruns += 1;
            }
            for o in out.iter_mut() {
                let s = c.samples.pop_front().unwrap_or(0) as i32;
                let scaled = (s * c.volume as i32) / 255;
                *o = (*o as i32 + scaled).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
        }
        out
    }

    pub fn underruns(&self) -> u32 {
        self.underruns
    }
}
