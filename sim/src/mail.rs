//! Modello dati del client mail, nel simulatore host. Nessuna rete reale.
//! Il client legge e scrive solo nella casella a capacità fissa; l'invio
//! passa dallo stack di rete (fuori da questo modulo).

/// Messaggi massimi in una casella.
pub const MAX_MESSAGES: usize = 64;
/// Dimensione massima del corpo (byte).
pub const MAX_BODY_BYTES: usize = 2048;
/// Dimensione massima dell'oggetto (byte).
pub const MAX_SUBJECT_BYTES: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub id: u32,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
    pub read: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailError {
    /// Casella piena.
    Full,
    /// Indirizzo non valido (serve `x@y.z`).
    BadAddress,
    /// Oggetto o corpo oltre il limite.
    TooLarge,
    /// Messaggio inesistente.
    NotFound,
}

/// Controllo minimo: una sola `@`, parte locale e dominio non vuoti, dominio con `.`.
pub fn valid_address(addr: &str) -> bool {
    let mut parts = addr.split('@');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(local), Some(domain), None) => {
            !local.is_empty()
                && !domain.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        }
        _ => false,
    }
}

#[derive(Debug, Default)]
pub struct Mailbox {
    messages: Vec<Message>,
    next_id: u32,
}

impl Mailbox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Aggiunge un messaggio (ricevuto o in bozza). Ritorna l'id assegnato.
    pub fn add(&mut self, from: &str, to: &str, subject: &str, body: &str) -> Result<u32, MailError> {
        if !valid_address(from) || !valid_address(to) {
            return Err(MailError::BadAddress);
        }
        if subject.len() > MAX_SUBJECT_BYTES || body.len() > MAX_BODY_BYTES {
            return Err(MailError::TooLarge);
        }
        if self.messages.len() >= MAX_MESSAGES {
            return Err(MailError::Full);
        }
        self.next_id += 1;
        let id = self.next_id;
        self.messages.push(Message {
            id,
            from: from.to_string(),
            to: to.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
            read: false,
        });
        Ok(id)
    }

    pub fn get(&self, id: u32) -> Option<&Message> {
        self.messages.iter().find(|m| m.id == id)
    }

    /// Segna come letto. Il contenuto non cambia.
    pub fn mark_read(&mut self, id: u32) -> Result<(), MailError> {
        let m = self.messages.iter_mut().find(|m| m.id == id).ok_or(MailError::NotFound)?;
        m.read = true;
        Ok(())
    }

    pub fn remove(&mut self, id: u32) -> Result<Message, MailError> {
        let pos = self.messages.iter().position(|m| m.id == id).ok_or(MailError::NotFound)?;
        Ok(self.messages.remove(pos))
    }

    pub fn unread_count(&self) -> usize {
        self.messages.iter().filter(|m| !m.read).count()
    }

    /// Messaggi ordinati dal più recente (id più alto).
    pub fn list(&self) -> Vec<&Message> {
        let mut v: Vec<&Message> = self.messages.iter().collect();
        v.sort_by(|a, b| b.id.cmp(&a.id));
        v
    }
}
