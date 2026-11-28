//! Regole di validazione dei campi del primo avvio.

pub const HOSTNAME_MAX: usize = 63;
pub const USER_MAX: usize = 32;
pub const TIMEZONE_MAX: usize = 40;
pub const PASSWORD_MIN: usize = 12;
pub const PASSWORD_MAX: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Empty,
    TooLong,
    BadChar,
    BadEdge,
    Reserved,
    PasswordTooShort,
    PasswordTooLong,
    PasswordIsUsername,
    PasswordMismatch,
    UnknownLanguage,
}

impl Error {
    pub fn message(self) -> &'static str {
        match self {
            Error::Empty => "Value is empty.",
            Error::TooLong => "Value is too long.",
            Error::BadChar => "Value contains a character that is not allowed.",
            Error::BadEdge => "Value cannot start or end with '-'.",
            Error::Reserved => "This name is reserved.",
            Error::PasswordTooShort => "Password must be at least 12 characters.",
            Error::PasswordTooLong => "Password is too long.",
            Error::PasswordIsUsername => "Password cannot be the same as the username.",
            Error::PasswordMismatch => "Passwords do not match.",
            Error::UnknownLanguage => "Unknown language. Use 'it' or 'en'.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    It,
    En,
}

impl Language {
    pub fn parse(s: &[u8]) -> Result<Self, Error> {
        match s {
            b"it" => Ok(Language::It),
            b"en" => Ok(Language::En),
            _ => Err(Error::UnknownLanguage),
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Language::It => "it",
            Language::En => "en",
        }
    }
}

/// Nome host: solo minuscole, cifre e trattini; non inizia né finisce con '-'.
pub fn check_hostname(s: &[u8]) -> Result<(), Error> {
    if s.is_empty() {
        return Err(Error::Empty);
    }
    if s.len() > HOSTNAME_MAX {
        return Err(Error::TooLong);
    }
    if s[0] == b'-' || s[s.len() - 1] == b'-' {
        return Err(Error::BadEdge);
    }
    if !s.iter().all(|&c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-') {
        return Err(Error::BadChar);
    }
    Ok(())
}

/// Nome utente: inizia con lettera minuscola o '_', poi lettere, cifre, '_' o '-'.
pub fn check_username(s: &[u8]) -> Result<(), Error> {
    if s.is_empty() {
        return Err(Error::Empty);
    }
    if s.len() > USER_MAX {
        return Err(Error::TooLong);
    }
    if s == b"root" {
        return Err(Error::Reserved);
    }
    if !(s[0].is_ascii_lowercase() || s[0] == b'_') {
        return Err(Error::BadChar);
    }
    if !s[1..]
        .iter()
        .all(|&c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-')
    {
        return Err(Error::BadChar);
    }
    Ok(())
}

/// Fuso orario in stile tzdata, es. "Europe/Rome". Controllo di forma, non di esistenza.
pub fn check_timezone(s: &[u8]) -> Result<(), Error> {
    if s.is_empty() {
        return Err(Error::Empty);
    }
    if s.len() > TIMEZONE_MAX {
        return Err(Error::TooLong);
    }
    if s[0] == b'/' || s.windows(2).any(|w| w == b"..") {
        return Err(Error::BadChar);
    }
    if !s
        .iter()
        .all(|&c| c.is_ascii_alphanumeric() || c == b'_' || c == b'/' || c == b'+' || c == b'-')
    {
        return Err(Error::BadChar);
    }
    Ok(())
}

/// Password: lunghezza minima 12 (linee guida NIST), diversa dal nome utente.
pub fn check_password(password: &[u8], username: &[u8]) -> Result<(), Error> {
    if password.len() < PASSWORD_MIN {
        return Err(Error::PasswordTooShort);
    }
    if password.len() > PASSWORD_MAX {
        return Err(Error::PasswordTooLong);
    }
    if password == username {
        return Err(Error::PasswordIsUsername);
    }
    Ok(())
}
