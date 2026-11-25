//! Gestore dei font (solo host): registro, attivazione, collezioni, font di sistema.
//! Limiti da verificare su GBA. Vedi `docs/font-manager.md`.

pub const MAX_FONTS: usize = 16;
pub const MAX_ACTIVE: usize = 4;
pub const MAX_COLLECTIONS: usize = 8;
pub const MAX_MEMBERS: usize = 8;
pub const MAX_NAME: usize = 32;
pub const MAX_SAMPLE: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontId(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Bitmap,
    Vector,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontError {
    Full,
    Duplicate,
    NameTooLong,
    Unknown,
    NotActive,
    TooManyActive,
    SystemFont,
    CollectionFull,
    MemberFull,
    SampleTooLong,
}

#[derive(Clone, Debug)]
struct Font {
    name: String,
    format: Format,
    active: bool,
}

#[derive(Clone, Debug)]
struct Collection {
    name: String,
    members: Vec<FontId>,
}

#[derive(Debug, Default)]
pub struct FontManager {
    fonts: Vec<Font>,
    collections: Vec<Collection>,
    system: Option<FontId>,
}

impl FontManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Installa un font (non attivo). Rifiuta nomi doppi o troppo lunghi.
    pub fn install(&mut self, name: &str, format: Format) -> Result<FontId, FontError> {
        if name.len() > MAX_NAME {
            return Err(FontError::NameTooLong);
        }
        if self.fonts.iter().any(|f| f.name == name) {
            return Err(FontError::Duplicate);
        }
        if self.fonts.len() >= MAX_FONTS {
            return Err(FontError::Full);
        }
        self.fonts.push(Font {
            name: name.to_string(),
            format,
            active: false,
        });
        Ok(FontId((self.fonts.len() - 1) as u8))
    }

    pub fn format(&self, id: FontId) -> Result<Format, FontError> {
        self.get(id).map(|f| f.format)
    }

    /// Anteprima: restituisce il testo di esempio se il font è installato.
    pub fn preview(&self, id: FontId, sample: &str) -> Result<String, FontError> {
        self.get(id)?;
        if sample.len() > MAX_SAMPLE {
            return Err(FontError::SampleTooLong);
        }
        Ok(sample.to_string())
    }

    pub fn activate(&mut self, id: FontId) -> Result<(), FontError> {
        let active_now = self.active().len();
        let font = self.get_mut(id)?;
        if font.active {
            return Ok(());
        }
        if active_now >= MAX_ACTIVE {
            return Err(FontError::TooManyActive);
        }
        font.active = true;
        Ok(())
    }

    /// Disattiva un font. Il font di sistema non si disattiva.
    pub fn deactivate(&mut self, id: FontId) -> Result<(), FontError> {
        if self.system == Some(id) {
            return Err(FontError::SystemFont);
        }
        self.get_mut(id)?.active = false;
        Ok(())
    }

    pub fn is_active(&self, id: FontId) -> bool {
        self.get(id).map(|f| f.active).unwrap_or(false)
    }

    pub fn active(&self) -> Vec<FontId> {
        (0..self.fonts.len())
            .filter(|&i| self.fonts[i].active)
            .map(|i| FontId(i as u8))
            .collect()
    }

    /// Imposta il font di sistema. Il font viene attivato se non lo è già.
    pub fn set_system(&mut self, id: FontId) -> Result<(), FontError> {
        self.activate(id)?;
        self.system = Some(id);
        Ok(())
    }

    pub fn system(&self) -> Option<FontId> {
        self.system
    }

    /// Crea una collezione vuota e ne restituisce l'indice.
    pub fn create_collection(&mut self, name: &str) -> Result<usize, FontError> {
        if name.len() > MAX_NAME {
            return Err(FontError::NameTooLong);
        }
        if self.collections.len() >= MAX_COLLECTIONS {
            return Err(FontError::CollectionFull);
        }
        self.collections.push(Collection {
            name: name.to_string(),
            members: Vec::new(),
        });
        Ok(self.collections.len() - 1)
    }

    /// Aggiunge un font installato a una collezione. Un font può stare in più collezioni.
    pub fn add_to_collection(&mut self, coll: usize, id: FontId) -> Result<(), FontError> {
        self.get(id)?;
        let c = self.collections.get_mut(coll).ok_or(FontError::Unknown)?;
        if c.members.contains(&id) {
            return Ok(());
        }
        if c.members.len() >= MAX_MEMBERS {
            return Err(FontError::MemberFull);
        }
        c.members.push(id);
        Ok(())
    }

    pub fn collection(&self, coll: usize) -> Option<(&str, &[FontId])> {
        self.collections
            .get(coll)
            .map(|c| (c.name.as_str(), c.members.as_slice()))
    }

    fn get(&self, id: FontId) -> Result<&Font, FontError> {
        self.fonts.get(id.0 as usize).ok_or(FontError::Unknown)
    }

    fn get_mut(&mut self, id: FontId) -> Result<&mut Font, FontError> {
        self.fonts.get_mut(id.0 as usize).ok_or(FontError::Unknown)
    }
}
