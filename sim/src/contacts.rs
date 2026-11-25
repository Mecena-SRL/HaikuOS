//! Modello dati della rubrica, nel simulatore host. Nessuna sincronizzazione reale.
//! Capacità fissa, come le altre strutture del monolite.

/// Contatti massimi in rubrica.
pub const MAX_CONTACTS: usize = 128;
/// Numeri o indirizzi per contatto.
pub const MAX_FIELDS: usize = 4;
/// Lunghezza massima del nome (byte).
pub const MAX_NAME_BYTES: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contact {
    pub id: u32,
    pub name: String,
    pub phones: Vec<String>,
    pub emails: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactError {
    /// Rubrica piena.
    Full,
    /// Nome vuoto o troppo lungo.
    BadName,
    /// Troppi numeri o indirizzi.
    TooManyFields,
    /// Telefono non valido (solo cifre, `+` iniziale ammesso, 3-20 caratteri).
    BadPhone,
    /// Indirizzo mail non valido.
    BadEmail,
    /// Contatto inesistente.
    NotFound,
}

fn valid_phone(p: &str) -> bool {
    let body = p.strip_prefix('+').unwrap_or(p);
    let len = p.chars().count();
    (3..=20).contains(&len) && !body.is_empty() && body.chars().all(|c| c.is_ascii_digit())
}

fn valid_email(e: &str) -> bool {
    crate::mail::valid_address(e)
}

#[derive(Debug, Default)]
pub struct Book {
    contacts: Vec<Contact>,
    next_id: u32,
}

impl Book {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.contacts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.contacts.is_empty()
    }

    fn check(name: &str, phones: &[String], emails: &[String]) -> Result<(), ContactError> {
        if name.trim().is_empty() || name.len() > MAX_NAME_BYTES {
            return Err(ContactError::BadName);
        }
        if phones.len() > MAX_FIELDS || emails.len() > MAX_FIELDS {
            return Err(ContactError::TooManyFields);
        }
        if phones.iter().any(|p| !valid_phone(p)) {
            return Err(ContactError::BadPhone);
        }
        if emails.iter().any(|e| !valid_email(e)) {
            return Err(ContactError::BadEmail);
        }
        Ok(())
    }

    pub fn add(&mut self, name: &str, phones: &[&str], emails: &[&str]) -> Result<u32, ContactError> {
        let phones: Vec<String> = phones.iter().map(|s| s.to_string()).collect();
        let emails: Vec<String> = emails.iter().map(|s| s.to_string()).collect();
        Self::check(name, &phones, &emails)?;
        if self.contacts.len() >= MAX_CONTACTS {
            return Err(ContactError::Full);
        }
        self.next_id += 1;
        let id = self.next_id;
        self.contacts.push(Contact { id, name: name.to_string(), phones, emails });
        Ok(id)
    }

    pub fn get(&self, id: u32) -> Option<&Contact> {
        self.contacts.iter().find(|c| c.id == id)
    }

    /// Sostituisce i dati di un contatto, con gli stessi controlli di `add`.
    pub fn update(&mut self, id: u32, name: &str, phones: &[&str], emails: &[&str]) -> Result<(), ContactError> {
        let phones: Vec<String> = phones.iter().map(|s| s.to_string()).collect();
        let emails: Vec<String> = emails.iter().map(|s| s.to_string()).collect();
        Self::check(name, &phones, &emails)?;
        let c = self.contacts.iter_mut().find(|c| c.id == id).ok_or(ContactError::NotFound)?;
        c.name = name.to_string();
        c.phones = phones;
        c.emails = emails;
        Ok(())
    }

    pub fn remove(&mut self, id: u32) -> Result<Contact, ContactError> {
        let pos = self.contacts.iter().position(|c| c.id == id).ok_or(ContactError::NotFound)?;
        Ok(self.contacts.remove(pos))
    }

    /// Ricerca per prefisso del nome, senza distinzione di maiuscole. Ordinata per nome.
    pub fn search(&self, prefix: &str) -> Vec<&Contact> {
        let p = prefix.to_lowercase();
        let mut v: Vec<&Contact> = self
            .contacts
            .iter()
            .filter(|c| c.name.to_lowercase().starts_with(&p))
            .collect();
        v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        v
    }

    /// Chi ha questo numero o indirizzo (per chiamate e mail in arrivo).
    pub fn find_by_address(&self, addr: &str) -> Option<&Contact> {
        self.contacts
            .iter()
            .find(|c| c.phones.iter().any(|p| p == addr) || c.emails.iter().any(|e| e == addr))
    }
}
