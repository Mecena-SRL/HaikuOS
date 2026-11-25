//! Modello dati dei promemoria, nel simulatore host. Il tempo è un numero
//! astratto (minuti) letto dall'orologio di sistema: qui non c'è timer reale.

/// Promemoria massimi.
pub const MAX_REMINDERS: usize = 32;
/// Lunghezza massima del testo (byte).
pub const MAX_TEXT_BYTES: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reminder {
    pub id: u32,
    pub text: String,
    /// Scadenza in minuti.
    pub due: u64,
    pub done: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReminderError {
    /// Lista piena.
    Full,
    /// Testo vuoto o troppo lungo.
    BadText,
    /// Promemoria inesistente.
    NotFound,
}

#[derive(Debug, Default)]
pub struct Reminders {
    items: Vec<Reminder>,
    next_id: u32,
}

impl Reminders {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn add(&mut self, text: &str, due: u64) -> Result<u32, ReminderError> {
        if text.trim().is_empty() || text.len() > MAX_TEXT_BYTES {
            return Err(ReminderError::BadText);
        }
        if self.items.len() >= MAX_REMINDERS {
            return Err(ReminderError::Full);
        }
        self.next_id += 1;
        let id = self.next_id;
        self.items.push(Reminder { id, text: text.to_string(), due, done: false });
        Ok(id)
    }

    pub fn complete(&mut self, id: u32) -> Result<(), ReminderError> {
        let r = self.items.iter_mut().find(|r| r.id == id).ok_or(ReminderError::NotFound)?;
        r.done = true;
        Ok(())
    }

    pub fn remove(&mut self, id: u32) -> Result<Reminder, ReminderError> {
        let pos = self.items.iter().position(|r| r.id == id).ok_or(ReminderError::NotFound)?;
        Ok(self.items.remove(pos))
    }

    /// Promemoria non completati scaduti entro `now`, dal più vecchio.
    pub fn due_at(&self, now: u64) -> Vec<&Reminder> {
        let mut v: Vec<&Reminder> = self.items.iter().filter(|r| !r.done && r.due <= now).collect();
        v.sort_by_key(|r| (r.due, r.id));
        v
    }

    /// Prossima scadenza di un promemoria non completato.
    pub fn next_due(&self) -> Option<u64> {
        self.items.iter().filter(|r| !r.done).map(|r| r.due).min()
    }
}
