//! Microkernel audio nel simulatore: mixer a canali con buffer fissi.
//! Campioni i16, mix con saturazione, conteggio degli underrun. Nessun hardware reale.
//! Regole: docs/resource-microkernels.md.

use crate::resource::{Arch, Error as QError, Kind, Manifest, Microkernel, Queue};

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

#[derive(Debug, PartialEq, Eq)]
pub struct Channel {
    /// Volume 0..=255. 255 = pieno.
    pub volume: u8,
    samples: Queue<i16>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AudioError {
    NoChannel,
    NoSlot,
    /// Buffer del canale pieno: il blocco intero viene rifiutato.
    Full,
}

pub struct Mixer {
    channels: Vec<Channel>,
    underruns: u32,
}

impl Mixer {
    /// `depth`: campioni per canale. Nessuna allocazione dopo `new`.
    pub fn new(channels: usize, depth: usize) -> Result<Self, AudioError> {
        if channels == 0 || channels > MAX_CHANNELS {
            return Err(AudioError::NoSlot);
        }
        let channels = (0..channels)
            .map(|_| Channel { volume: 255, samples: Queue::new(depth) })
            .collect();
        Ok(Self { channels, underruns: 0 })
    }

    /// Accoda campioni: o entrano tutti, o nessuno.
    pub fn push(&mut self, ch: usize, samples: &[i16]) -> Result<(), AudioError> {
        let c = self.channels.get_mut(ch).ok_or(AudioError::NoChannel)?;
        if samples.len() > c.samples.free() {
            return Err(AudioError::Full);
        }
        for &x in samples {
            c.samples.push(x).map_err(|_: QError| AudioError::Full)?;
        }
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
                let s = c.samples.pop().unwrap_or(0) as i32;
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

impl Microkernel for Mixer {
    fn manifest(&self) -> Manifest {
        AUDIO_MANIFEST
    }
}
