//! Gestore dei font sulla ROM (prototipo). Registro fisso, nessuna allocazione.
//! Stesse regole di `sim/src/fonts.rs`: max font attivi, font di sistema non disattivabile.
//! Anteprima: il solo font in ROM è il 3×5 di `glyph.rs`. Vedi `docs/font-manager.md`.

#![allow(dead_code)]
use crate::glyph;

pub const MAX: usize = 4;
pub const MAX_ACTIVE: usize = 4;
pub const NAME: usize = 12;

#[derive(Clone, Copy)]
struct Font {
    name: [u8; NAME],
    len: u8,
    active: bool,
}

#[derive(Clone, Copy)]
pub struct FontMgr {
    fonts: [Option<Font>; MAX],
    system: Option<u8>,
}

impl FontMgr {
    pub const fn new() -> Self {
        FontMgr { fonts: [None; MAX], system: None }
    }

    /// Installa un font (non attivo). Rifiuta nomi vuoti o troppo lunghi, nomi doppi, registro pieno.
    pub fn install(&mut self, name: &[u8]) -> Result<u8, ()> {
        if name.is_empty() || name.len() > NAME || self.find(name).is_some() {
            return Err(());
        }
        let slot = self.fonts.iter().position(|f| f.is_none()).ok_or(())?;
        let mut buf = [0u8; NAME];
        buf[..name.len()].copy_from_slice(name);
        self.fonts[slot] = Some(Font { name: buf, len: name.len() as u8, active: false });
        Ok(slot as u8)
    }

    pub fn find(&self, name: &[u8]) -> Option<u8> {
        self.fonts
            .iter()
            .position(|f| matches!(f, Some(f) if &f.name[..f.len as usize] == name))
            .map(|i| i as u8)
    }

    pub fn name(&self, i: u8) -> Option<&[u8]> {
        self.fonts.get(i as usize)?.as_ref().map(|f| &f.name[..f.len as usize])
    }

    pub fn is_active(&self, i: u8) -> bool {
        matches!(self.fonts.get(i as usize), Some(Some(f)) if f.active)
    }

    pub fn active_count(&self) -> usize {
        self.fonts.iter().flatten().filter(|f| f.active).count()
    }

    pub fn activate(&mut self, i: u8) -> Result<(), ()> {
        if self.is_active(i) {
            return Ok(());
        }
        if self.active_count() >= MAX_ACTIVE {
            return Err(());
        }
        match self.fonts.get_mut(i as usize) {
            Some(Some(f)) => {
                f.active = true;
                Ok(())
            }
            _ => Err(()),
        }
    }

    /// Disattiva un font. Il font di sistema non si disattiva.
    pub fn deactivate(&mut self, i: u8) -> Result<(), ()> {
        if self.system == Some(i) {
            return Err(());
        }
        match self.fonts.get_mut(i as usize) {
            Some(Some(f)) => {
                f.active = false;
                Ok(())
            }
            _ => Err(()),
        }
    }

    /// Imposta il font di sistema; lo attiva se serve.
    pub fn set_system(&mut self, i: u8) -> Result<(), ()> {
        self.activate(i)?;
        self.system = Some(i);
        Ok(())
    }

    pub fn system(&self) -> Option<u8> {
        self.system
    }

    /// Anteprima: disegna il testo con il font 3×5 di ROM nel framebuffer.
    pub fn preview(&self, px: &mut [u16], w: usize, h: usize, x: usize, y: usize, sample: &[u8], color: u16) {
        glyph::draw_text(px, w, h, x, y, sample, color);
    }
}
