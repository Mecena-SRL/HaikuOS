//! Procedura guidata del primo avvio. Non conosce l'hardware: parla solo
//! tramite il trait `Console`, così è testabile sull'host.

use crate::config::{self, Error, Language};
use crate::password::{Hash, ITERATIONS, SALT_LEN};
use crate::text::{wipe_bytes, Text};

pub const LINE_MAX: usize = 256;

/// Canale di input/output. `read_line` restituisce i byte letti in `buf`
/// (senza fine riga). Se `secret` è vero, il console non deve mostrare il testo.
pub trait Console {
    fn write(&mut self, s: &str);
    fn read_line(&mut self, buf: &mut [u8], secret: bool) -> usize;
}

pub struct Setup {
    pub hostname: Text<{ config::HOSTNAME_MAX }>,
    pub language: Language,
    pub timezone: Text<{ config::TIMEZONE_MAX }>,
    pub username: Text<{ config::USER_MAX }>,
    pub password_hash: Hash,
}

impl Setup {
    /// Scrive la configurazione da salvare. La password compare solo come hash.
    pub fn render(&self, c: &mut impl Console) {
        c.write("hostname=");
        c.write(self.hostname.as_str());
        c.write("\nlanguage=");
        c.write(self.language.code());
        c.write("\ntimezone=");
        c.write(self.timezone.as_str());
        c.write("\nadmin=");
        c.write(self.username.as_str());
        c.write("\nadmin_password=");
        c.write(self.password_hash.encode().as_str());
        c.write("\n");
    }
}

/// Esegue il primo avvio con i parametri di default (`ITERATIONS`).
pub fn run(c: &mut impl Console, salt: &[u8; SALT_LEN]) -> Setup {
    run_with(c, salt, ITERATIONS)
}

/// Come `run`, con numero di iterazioni scelto dal chiamante (utile nei test).
pub fn run_with(c: &mut impl Console, salt: &[u8; SALT_LEN], iterations: u32) -> Setup {
    let hostname = ask(c, "Hostname: ", false, config::check_hostname);
    let language = loop {
        let line = read(c, "Language (it/en): ", false);
        match Language::parse(line.as_bytes()) {
            Ok(l) => break l,
            Err(e) => error(c, e),
        }
    };
    let timezone = ask(c, "Timezone (es. Europe/Rome): ", false, config::check_timezone);
    let username = ask(c, "Admin username: ", false, config::check_username);

    let mut password = loop {
        let mut pw = read(c, "Admin password: ", true);
        match config::check_password(pw.as_bytes(), username.as_bytes()) {
            Ok(()) => {
                let mut confirm = read(c, "Repeat password: ", true);
                let same = confirm == pw;
                confirm.wipe();
                if same {
                    break pw;
                }
                error(c, Error::PasswordMismatch);
            }
            Err(e) => error(c, e),
        }
        pw.wipe();
    };
    let password_hash = Hash::new(password.as_bytes(), salt, iterations);
    password.wipe();

    Setup { hostname, language, timezone, username, password_hash }
}

/// Chiede finché la risposta non passa il controllo `check`.
fn ask<const N: usize>(
    c: &mut impl Console,
    prompt: &str,
    secret: bool,
    check: fn(&[u8]) -> Result<(), Error>,
) -> Text<N> {
    loop {
        let line = read(c, prompt, secret);
        match check(line.as_bytes()) {
            Ok(()) => {
                if let Some(t) = Text::from_bytes(line.as_bytes()) {
                    return t;
                }
                error(c, Error::TooLong);
            }
            Err(e) => error(c, e),
        }
    }
}

/// Legge una riga, con spazi iniziali e finali rimossi.
fn read(c: &mut impl Console, prompt: &str, secret: bool) -> Text<LINE_MAX> {
    c.write(prompt);
    let mut buf = [0u8; LINE_MAX];
    let n = c.read_line(&mut buf, secret);
    let t = Text::from_bytes(trim(&buf[..n.min(LINE_MAX)])).unwrap_or_else(Text::empty);
    wipe_bytes(&mut buf);
    t
}

fn trim(s: &[u8]) -> &[u8] {
    let start = s.iter().position(|&b| b != b' ').unwrap_or(s.len());
    let end = s.iter().rposition(|&b| b != b' ').map_or(start, |i| i + 1);
    &s[start..end]
}

fn error(c: &mut impl Console, e: Error) {
    c.write("Error: ");
    c.write(e.message());
    c.write("\n");
}
