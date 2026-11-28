//! Lettore audio nel simulatore: manda PCM i16 mono a un canale del mixer, a blocchi.
//! Nessuna decodifica (niente mp3/ogg): solo campioni grezzi. Regole: docs/apps-media.md.

use crate::audio::{AudioError, Mixer};

pub struct Player {
    ch: usize,
    data: Vec<i16>,
    pos: usize,
    playing: bool,
}

impl Player {
    pub fn new(ch: usize, data: Vec<i16>) -> Self {
        Player { ch, data, pos: 0, playing: false }
    }

    pub fn play(&mut self) {
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn is_finished(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Accoda al mixer quanti campioni entrano nel buffer del canale. Ritorna quanti.
    /// In pausa o a fine brano non accoda nulla.
    pub fn feed(&mut self, mixer: &mut Mixer) -> Result<usize, AudioError> {
        if !self.playing || self.is_finished() {
            return Ok(0);
        }
        let n = mixer.free(self.ch)?.min(self.data.len() - self.pos);
        mixer.push(self.ch, &self.data[self.pos..self.pos + n])?;
        self.pos += n;
        Ok(n)
    }
}
