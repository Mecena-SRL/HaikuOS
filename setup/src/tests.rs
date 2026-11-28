use crate::config::*;
use crate::password::Hash;
use crate::wizard::{run_with, Console};
use std::string::String;
use std::vec::Vec;

const SALT: [u8; 16] = [7; 16];
const FAST: u32 = 1_000;

#[test]
fn hostname_rules() {
    assert!(check_hostname(b"haiku-01").is_ok());
    assert_eq!(check_hostname(b""), Err(Error::Empty));
    assert_eq!(check_hostname(b"-bad"), Err(Error::BadEdge));
    assert_eq!(check_hostname(b"Haiku"), Err(Error::BadChar));
    assert_eq!(check_hostname(&[b'a'; 64]), Err(Error::TooLong));
}

#[test]
fn username_rules() {
    assert!(check_username(b"ivan").is_ok());
    assert!(check_username(b"_svc").is_ok());
    assert_eq!(check_username(b"root"), Err(Error::Reserved));
    assert_eq!(check_username(b"1ivan"), Err(Error::BadChar));
    assert_eq!(check_username(b"Ivan"), Err(Error::BadChar));
}

#[test]
fn timezone_rules() {
    assert!(check_timezone(b"Europe/Rome").is_ok());
    assert_eq!(check_timezone(b"../etc"), Err(Error::BadChar));
    assert_eq!(check_timezone(b"/Rome"), Err(Error::BadChar));
}

#[test]
fn password_rules() {
    assert!(check_password(b"correct horse 9", b"ivan").is_ok());
    assert_eq!(check_password(b"short", b"ivan"), Err(Error::PasswordTooShort));
    assert_eq!(check_password(b"ivanivanivan", b"ivanivanivan"), Err(Error::PasswordIsUsername));
}

#[test]
fn hash_verifies_and_hides_password() {
    let h = Hash::new(b"correct horse 9", &SALT, FAST);
    assert!(h.verify(b"correct horse 9"));
    assert!(!h.verify(b"correct horse 8"));
    let enc = h.encode();
    assert!(enc.as_str().starts_with("pbkdf2-sha256$1000$"));
    assert!(!enc.as_str().contains("correct"));
}

#[test]
fn same_password_different_salt_differs() {
    let a = Hash::new(b"correct horse 9", &[1; 16], FAST);
    let b = Hash::new(b"correct horse 9", &[2; 16], FAST);
    assert_ne!(a.digest, b.digest);
}

/// Console di test: risponde con le righe in coda e registra l'output.
struct Script {
    lines: Vec<&'static str>,
    out: String,
}

impl Console for Script {
    fn write(&mut self, s: &str) {
        self.out.push_str(s);
    }
    fn read_line(&mut self, buf: &mut [u8], _secret: bool) -> usize {
        let line = self.lines.remove(0).as_bytes();
        buf[..line.len()].copy_from_slice(line);
        line.len()
    }
}

#[test]
fn wizard_retries_until_valid_and_builds_setup() {
    let mut c = Script {
        lines: vec![
            "-bad",            // hostname non valido
            "haiku-box",       // hostname ok
            "fr",              // lingua non valida
            "it",
            "Europe/Rome",
            "root",            // riservato
            "ivan",
            "ivan",            // password uguale al nome
            "short",           // troppo corta
            "correct horse 9",
            "altra password 9", // non coincide
            "correct horse 9",
            "correct horse 9",
        ],
        out: String::new(),
    };
    let s = run_with(&mut c, &SALT, FAST);
    assert_eq!(s.hostname.as_str(), "haiku-box");
    assert_eq!(s.language, Language::It);
    assert_eq!(s.timezone.as_str(), "Europe/Rome");
    assert_eq!(s.username.as_str(), "ivan");
    assert!(s.password_hash.verify(b"correct horse 9"));
    assert!(c.lines.is_empty());

    let mut out = Script { lines: vec![], out: String::new() };
    s.render(&mut out);
    assert!(out.out.contains("admin=ivan\n"));
    assert!(out.out.contains("admin_password=pbkdf2-sha256$1000$"));
    assert!(!out.out.contains("correct horse"));
}
